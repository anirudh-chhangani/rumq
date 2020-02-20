use std::collections::HashMap;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use derive_more::From;
use rumq_core::mqtt4::{Packet, Publish};

#[derive(Debug, From)]
pub enum Error {
    Time(SystemTimeError),
}

#[derive(Debug)]
pub struct Messages {
    // current segment id
    pub segment_id: u64,
    // current log offset
    pub log_offset: usize,
    // packets
    pub packets: Vec<Publish>,
}

#[derive(Debug)]
struct Segment {
    // id of this segment
    pub id: u64,
    // current size of this segment
    pub current_size: usize,
    // max_allowed size of the segment
    max_size: usize,
    // timestamp when the log is created
    timestamp: u128,
    // all the packets in this segment
    packets: Vec<Publish>,
}

impl Segment {
    pub fn new(id: u64, max_size: usize) -> Result<Segment, Error> {
        let segment = Segment {
            id,
            current_size: 0,
            max_size,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            packets: Vec::new(),
        };

        Ok(segment)
    }

    // Fills the segment with given publish. If the segment is full, returns
    // Some(offset of the last element) in the segment
    pub fn fill(&mut self, pubilsh: Publish) -> Option<u64> {
        let payload_size = pubilsh.payload.len();
        self.packets.push(pubilsh);
        self.current_size += payload_size;

        if self.current_size >= self.max_size {
            return Some(self.packets.len() as u64 - 1);
        }

        None
    }
}

#[derive(Debug)]
pub struct CommitLog {
    partitions: HashMap<String, Vec<Segment>>,
    max_segement_size: usize,
    segments_per_partition: usize,
    current_segment_id: u64,
}

impl CommitLog {
    pub fn new(max_segement_size: usize, segments_per_partition: usize) -> Result<CommitLog, Error> {
        let commitlog =
            CommitLog { partitions: HashMap::new(), max_segement_size, segments_per_partition, current_segment_id: 0 };

        Ok(commitlog)
    }

    pub fn fill(&mut self, publish: Publish) -> Result<(), Error> {
        if let Some(segments) = self.partitions.get_mut(&publish.topic_name) {
            // fill the last segment of the partition. Always exists and this is the only function
            // which can delete segment
            let segment = segments.last_mut().unwrap();
            if let Some(_) = segment.fill(publish) {
                // delete the first segment when the number of segments are at limit
                if segments.len() >= self.segments_per_partition {
                    segments.remove(0);
                }

                // push a new segment
                self.current_segment_id += 1;
                let segment = Segment::new(self.current_segment_id, self.max_segement_size)?;
                segments.push(segment);
            }
        } else {
            // create a new partition with this new topic
            let topic = publish.topic_name.clone();
            let mut partition = Vec::new();
            let mut segment = Segment::new(0, self.max_segement_size)?;
            segment.fill(publish);
            partition.push(segment);
            self.partitions.insert(topic, partition);
        }

        Ok(())
    }

    // get a maximum of n elements from partition's segments
    // return's the segment id and log offset of the last element of the batch
    pub fn get(&self, topic: &str, segment_id: u64, mut log_offset: usize, mut count: usize) -> Option<Messages> {
        let segments = match self.partitions.get(topic) {
            Some(segments) => segments,
            None => return None,
        };

        // segment ids are ordered. find the index of the segment with the given id
        let segment_index = match segments.iter().next() {
            Some(first_segment) if first_segment.id >= segment_id => 0,
            Some(first_segment) => segment_id - first_segment.id,
            None => return None,
        };

        // the difference betweeen 2 elements of vec cannot be greater than usize. ok to cast
        let segment_index = segment_index as usize;

        // ignore cases where segment index crosses range
        if segment_index >= segments.len() {
            return None;
        }

        // fill publishes
        let mut messages = Messages { segment_id: 0, log_offset: 0, packets: Vec::new() };
        for segment in segments.split_at(segment_index).1.iter() {
            // continue to next segment if the given log offset doesn't exist
            if segment.packets.get(log_offset).is_none() {
                log_offset = 0;
                continue;
            }

            let o: Vec<Publish> = segment.packets.split_at(log_offset).1.iter().take(count).cloned().collect();
            let collected_message_count = o.len();
            messages.segment_id = segment.id;
            messages.log_offset = log_offset + o.len() - 1;
            messages.packets.extend(o);

            if collected_message_count >= count {
                return Some(messages);
            }

            // decrease the collection count for the next iteration
            count = count - collected_message_count;
        }

        if messages.packets.len() > 0 {
            Some(messages)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::{CommitLog, Segment};
    use rumq_core::mqtt4::{publish, QoS};

    #[test]
    fn filled_segment_returns_correct_offset() {
        let mut segment = Segment::new(0, 1024 * 3).unwrap();
        let payload = vec![0; 1024];

        let publish = publish("hello/world", QoS::AtLeastOnce, payload);

        assert!(segment.fill(publish.clone()).is_none());
        assert!(segment.fill(publish.clone()).is_none());

        match segment.fill(publish.clone()) {
            Some(offset) => assert_eq!(2, offset),
            None => panic!("Segment should've been full by now"),
        }
    }

    #[test]
    fn commit_log_fills_correctly() {
        // 10 segments. Each segment size = 10K
        let mut commitlog = CommitLog::new(10 * 1024, 10).unwrap();

        let payload = vec![0; 1024];
        let publish = publish("hello/world", QoS::AtLeastOnce, payload);

        // fill 1000 1k publishes
        for _ in 0..1000 {
            commitlog.fill(publish.clone()).unwrap()
        }

        let partition = commitlog.partitions.get("hello/world").unwrap();
        assert_eq!(partition.len(), 10);
    }

    #[test]
    fn commitlog_returns_data_and_offset_correctly() {
        // max 10 segments. Each segment size = 10K. Max 100KB in total
        let mut commitlog = CommitLog::new(10 * 1024, 10).unwrap();

        let o = commitlog.get("hello/world", 0, 0, 10);
        assert!(o.is_none());

        let payload = vec![0; 1024];
        let publish = publish("hello/world", QoS::AtLeastOnce, payload);

        // fill 100 1k publishes. distibuted as 10 segments with size 10K each
        // deletes segment 0 when segment 9 is completely full and inserts an empty segment 10.
        // So segments 1..10 from indexes 0..9
        for _ in 0..100 {
            commitlog.fill(publish.clone()).unwrap()
        }

        // go past the segments boundary
        let o = commitlog.get("hello/world", 12, 0, 10);
        assert!(o.is_none());

        let o = commitlog.get("hello/world", 0, 0, 10).unwrap();
        assert_eq!(o.segment_id, 1);
        assert_eq!(o.log_offset, 9);

        let o = commitlog.get("hello/world", o.segment_id, o.log_offset + 1, 10).unwrap();
        assert_eq!(o.segment_id, 2);
        assert_eq!(o.log_offset, 9);

        // cross the boundary
        let o = commitlog.get("hello/world", o.segment_id, o.log_offset + 1, 15).unwrap();
        assert_eq!(o.segment_id, 4);
        assert_eq!(o.log_offset, 4);

        // go to the end
        let o = commitlog.get("hello/world", o.segment_id, o.log_offset + 1, 1000).unwrap();
        assert_eq!(o.segment_id, 9);
        assert_eq!(o.log_offset, 9);
    }
}
