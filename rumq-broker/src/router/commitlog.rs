use std::collections::HashMap;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use derive_more::From;
use rumq_core::mqtt4::Publish;

#[derive(Debug, From)]
pub enum Error {
    Time(SystemTimeError),
}

#[derive(Debug)]
pub struct Messages {
    // current segment offset
    pub segment_offset: u64,
    // current log offset
    pub log_offset: u64,
    // packets
    pub packets: Vec<Publish>,
}

#[derive(Debug)]
struct Segment {
    // starting offset of this segment
    pub offset: u64,
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
    pub fn new(offset: u64, max_size: usize) -> Result<Segment, Error> {
        let segment = Segment {
            offset,
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
    previous_segment_offset: u64,
}

impl CommitLog {
    pub fn new(max_segement_size: usize, segments_per_partition: usize) -> Result<CommitLog, Error> {
        let commitlog =
            CommitLog { partitions: HashMap::new(), max_segement_size, segments_per_partition, previous_segment_offset: 0 };

        Ok(commitlog)
    }

    pub fn fill(&mut self, publish: Publish) -> Result<(), Error> {
        if let Some(segments) = self.partitions.get_mut(&publish.topic_name) {
            // fill the last segment of the partition. Always exists and this is the only function
            // which can delete segment
            let segment = segments.last_mut().unwrap();
            if let Some(offset) = segment.fill(publish) {
                // delete the first segment when the number of segments are at limit
                if segments.len() >= self.segments_per_partition {
                    segments.remove(0);
                }

                // push a new segment
                self.previous_segment_offset += offset;
                let segment = Segment::new(self.previous_segment_offset + 1, self.max_segement_size)?;
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

    // get a maximum of n elements from the partitions
    pub fn get(&self, topic: &str, segment_offset: usize, log_offset: usize, count: usize) -> Option<Messages> {
        let segments = match self.partitions.get(topic) {
            Some(segments) => segments,
            None => return None,
        };

        let mut messages = Messages { segment_offset: 0, log_offset: 0, packets: Vec::new() };
        for segment in segments.split_at(segment_offset).1.iter() {
            let max_offset = segment.packets.len();

            // starting offset should not exceed max offset of the segment
            if log_offset >= max_offset {
                return None;
            }

            let o: Vec<Publish> = segment.packets.split_at(log_offset).1.iter().take(count).cloned().collect();
            messages.log_offset = (log_offset + o.len()) as u64;
            messages.packets.extend(o);
        }

        None
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
        for i in 0..1000 {
            commitlog.fill(publish.clone()).unwrap()
        }

        let partition = commitlog.partitions.get("hello/world").unwrap();
        assert_eq!(partition.len(), 10);
    }
}
