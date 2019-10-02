use std::collections::HashMap;
use std::string;

use std::ops::Add;
use std::ops::Sub;

use chrono::prelude::*;
use chrono::Duration;
use regex::Regex;

use crate::emoji::{self, Emojis};
use crate::message::{Message, MessageError, MessageErrorKind, Result};

#[derive(Clone, Copy, Serialize, Debug)]
pub enum TimelineType {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl TimelineType {
    fn as_days(&self) -> u32 {
        match self {
            TimelineType::Daily => 1,
            TimelineType::Weekly => 7,
            TimelineType::Monthly => 31,
            TimelineType::Yearly => 365,
        }
    }

    fn duration(&self) -> Duration {
        match self {
            TimelineType::Daily => Duration::days(1),
            TimelineType::Weekly => Duration::weeks(1),
            TimelineType::Monthly => Duration::days(31),
            TimelineType::Yearly => Duration::days(365),
        }
    }

    fn start_of(&self, date: &NaiveDateTime) -> NaiveDateTime {
        match self {
            TimelineType::Daily => date.sub(Duration::seconds(i64::from(
                date.num_seconds_from_midnight(),
            ))),
            TimelineType::Weekly => date
                .sub(Duration::seconds(i64::from(
                    date.num_seconds_from_midnight(),
                )))
                .sub(Duration::days(i64::from(
                    date.weekday().num_days_from_monday(),
                ))),
            TimelineType::Monthly => NaiveDateTime::parse_from_str(
                format!("{}-{:02}-01T00:00:00", date.year(), date.month()).as_str(),
                "%Y-%m-%dT%H:%M:%S",
            )
            .expect("fail to calculate beginning of the month"),
            TimelineType::Yearly => NaiveDateTime::parse_from_str(
                format!(
                    "{}-01-01T00:00:00",
                    if date.month() == 12 {
                        date.year() + 1
                    } else {
                        date.year()
                    },
                )
                .as_str(),
                "%Y-%m-%dT%H:%M:%S",
            )
            .expect("fail to calculate beginning of the year"),
        }
    }
}

impl string::ToString for TimelineType {
    fn to_string(&self) -> String {
        match self {
            TimelineType::Daily => String::from("daily"),
            TimelineType::Weekly => String::from("weekly"),
            TimelineType::Monthly => String::from("monthly"),
            TimelineType::Yearly => String::from("yearly"),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Stats<T> {
    messages: T,
    words: T,
    letters: T,
}

impl Stats<f32> {
    fn calc_average(cnv: &Conversation, period: TimelineType) -> Self {
        let period = period.as_days() as f32;

        let messages = cnv.count() as f32 / period;
        let words = cnv.words() as f32 / period;
        let letters = cnv.letters() as f32 / period;

        Self {
            messages,
            words,
            letters,
        }
    }
}

impl Stats<usize> {
    fn calc_total(cnv: &Conversation) -> Self {
        Self {
            messages: cnv.count(),
            words: cnv.words(),
            letters: cnv.letters(),
        }
    }
}

pub type Frequency = HashMap<String, u32>;
pub type DateTimeHashMap<T> = HashMap<NaiveDateTime, T>;

pub type TimelineMap = DateTimeHashMap<Conversation>;

#[derive(Serialize, Clone, Debug)]
pub struct ParticipantStats {
    total: Stats<usize>,
    average: Stats<f32>,
}

pub type ParticipantMap = HashMap<String, ParticipantStats>;

#[derive(Serialize, Clone, Debug)]
pub struct TimelineStats {
    total: Stats<usize>,
    average: Stats<f32>,

    period: TimelineType,
    participants: ParticipantMap,
}

#[derive(Serialize, Clone, Debug)]
pub struct Timeline(DateTimeHashMap<TimelineStats>);

impl Into<DateTimeHashMap<TimelineStats>> for Timeline {
    fn into(self) -> DateTimeHashMap<TimelineStats> {
        self.0
    }
}

impl Timeline {
    fn new(src: TimelineMap, period: TimelineType) -> Self {
        let mut map = HashMap::new();

        for (dt, cnv) in src {
            let total = Stats::<usize>::calc_total(&cnv);
            let average = Stats::<f32>::calc_average(&cnv, period);

            let mut participants: ParticipantMap = HashMap::new();

            for p in cnv.participants() {
                let p_cnv = cnv.by_author(p.to_string());

                let p_total = Stats::<usize>::calc_total(&p_cnv);
                let p_average = Stats::<f32>::calc_average(&p_cnv, period);

                participants.insert(
                    p.to_string(),
                    ParticipantStats {
                        total: p_total,
                        average: p_average,
                    },
                );
            }

            map.insert(
                dt,
                TimelineStats {
                    total,
                    average,
                    participants,
                    period: period,
                },
            );
        }

        Timeline(map)
    }
}

#[derive(Debug, Clone)]
pub struct Conversation {
    messages: Vec<Message>,
    participants: Vec<String>,
}

#[allow(dead_code)]
impl Conversation {
    pub fn from_str(raw: &str) -> Result<Conversation> {
        lazy_static! {
            static ref PATTERN:Regex = Regex::new(r"\[(?P<datetime>\d{4}-\d{2}-\d{2},\s\d{2}:\d{2}:\d{2})\]\s(?P<author>.*?):\s(?P<text>.*)").expect("invalid regex");
        }

        let mut messages: Vec<Message> = Vec::new();
        let mut participants: Vec<String> = Vec::new();

        for capture in PATTERN.captures_iter(&raw) {
            if capture["text"]
                .contains("Messages to this group are now secured with end-to-end encryption")
            {
                continue;
            }

            let message = match Message::from_str(
                &capture["datetime"],
                &capture["author"],
                &capture["text"].trim(),
            ) {
                Ok(message) => message,
                Err(e) => return Err(e),
            };
            if !participants.contains(&message.author) {
                participants.push(message.author.clone());
            }

            messages.push(message);
        }

        Ok(Conversation {
            messages,
            participants,
        })
    }

    pub fn new(messages: Vec<Message>, participants: Vec<String>) -> Conversation {
        Conversation {
            messages,
            participants,
        }
    }

    pub fn first(&self) -> Option<&Message> {
        self.messages.first()
    }

    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn duration(&self) -> Result<Duration> {
        let first = match self.first() {
            Some(v) => v,
            None => return Err(MessageError(MessageErrorKind::EmptyMessage)),
        };
        let last = match self.last() {
            Some(v) => v,
            None => return Err(MessageError(MessageErrorKind::EmptyMessage)),
        };

        Ok(last.datetime.sub(first.datetime))
    }

    pub fn count(&self) -> usize {
        self.messages.len()
    }

    fn combine_raw(&self) -> String {
        let mut text = String::new();

        for message in self.messages.iter() {
            text.push_str(message.text.as_str());
        }

        text.to_string()
    }
    fn combine(&self) -> String {
        let mut text = self.combine_raw();

        text.retain(|c| c.is_ascii());

        text.trim().to_string()
    }

    pub fn words(&self) -> usize {
        self.combine().split_whitespace().count()
    }

    pub fn letters(&self) -> usize {
        let words: Vec<String> = self
            .combine()
            .split_whitespace()
            .map(|word| String::from(word))
            .collect();

        words.join("").len()
    }

    // average letters and words per message
    pub fn average(&self) -> (f32, f32) {
        let mut l = 0;
        let mut w = 0;

        for message in self.messages.iter() {
            l += message.letters().len();
            w += message.words().len();
        }

        let w = w as f32;
        let l = l as f32;

        let c = self.count() as f32;

        (w / c, l / c)
    }

    pub fn participants(&self) -> &Vec<String> {
        &self.participants
    }

    pub fn by_author(&self, author: String) -> Conversation {
        Conversation {
            messages: self
                .messages
                .clone()
                .into_iter()
                .filter(|m| m.author.eq(&author))
                .collect(),
            participants: vec![author],
        }
    }

    pub fn by_range(&self, start: NaiveDateTime, end: NaiveDateTime) -> Conversation {
        Conversation {
            messages: self
                .messages
                .clone()
                .into_iter()
                .filter(|m| m.datetime >= start && m.datetime < end)
                .collect(),
            participants: self.participants().clone(),
        }
    }

    pub fn emojis(&self) -> Emojis {
        emoji::count(&self.combine_raw())
    }

    pub fn frequency(&self) -> Frequency {
        let mut map = HashMap::new();
        for n in 0..24 {
            map.insert(format!("{:02}h", n), 0);
        }

        for m in self.messages.iter() {
            let hour = format!("{:02}h", m.datetime.hour());
            if let Some(val) = map.get_mut(&hour) {
                *val += 1;
            }
        }

        map
    }

    fn timeline_map(&self, kind: TimelineType) -> TimelineMap {
        let first = self.first().unwrap().datetime;
        let last = self.last().unwrap().datetime;

        let mut cursor = kind.start_of(&first);

        let mut timeline = HashMap::new();

        loop {
            if cursor > last {
                break;
            }

            let next = kind.start_of(&cursor.add(kind.duration()));

            timeline.insert(cursor, self.by_range(cursor, next));

            cursor = next;
        }

        timeline
    }

    pub fn timeline(&self, kind: TimelineType) -> Timeline {
        Timeline::new(self.timeline_map(kind), kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static MOCK: &str = r"
[2001-01-19, 02:34:56] Foo: Hey! 💩
[2001-01-21, 02:34:56] Bar Baz: heyyyyyyy, 'sup
";

    #[test]
    fn from_str_parses_messages() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.messages.len(), 2);
    }

    #[test]
    fn from_str_parses_partipants() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.participants.len(), 2);
        assert_eq!(c.participants, vec!["Foo", "Bar Baz"]);
    }
    #[test]
    fn first_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.first().unwrap().text, "Hey! 💩");
    }

    #[test]
    fn last_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.last().unwrap().text, "heyyyyyyy, 'sup");
    }

    #[test]
    fn duration_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.duration().unwrap().num_days(), 2);
    }

    #[test]
    fn combine_raw_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.combine_raw(), "Hey! 💩heyyyyyyy, \'sup");
    }
    #[test]
    fn combine_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.combine(), "Hey! heyyyyyyy, \'sup");
    }

    #[test]
    fn words_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.words(), 3);
    }

    #[test]
    fn letters_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.letters(), 18);
    }

    #[test]
    fn average_works() {
        let c = Conversation::from_str(MOCK).unwrap();
        assert_eq!(c.average(), (1.5, 7.5));
    }

    #[test]
    fn by_author_works() {
        let c = Conversation::from_str(MOCK).unwrap();

        let c0 = c.by_author(String::from("Foo"));
        let c1 = c.by_author(String::from("Bar Baz"));

        assert_eq!(c0.messages.len(), 1);
        assert_eq!(c0.participants, vec!["Foo"]);
        assert_eq!(c1.messages.len(), 1);
        assert_eq!(c1.participants, vec!["Bar Baz"]);
    }

    #[test]
    fn emojis_works() {
        let c = Conversation::from_str(MOCK).unwrap();

        assert_eq!(c.emojis().keys().len(), 1);
        assert_eq!(c.emojis().keys().nth(0), Some(&String::from("💩")));
    }

    #[test]
    fn frequence_works() {
        let mock_for_frequency = r"
[2001-01-19, 00:34:56] Foo: Test
[2001-01-19, 02:00:00] Bar: What?
[2001-01-19, 02:59:59] Foo: I said, test?
[2001-01-19, 03:34:56] Bar: What?
[2001-01-19, 04:34:56] Foo: Nevermind
[2001-01-19, 05:34:56] Foo: There? 
[2001-01-19, 06:34:56] Bar: Yes?
[2001-01-19, 23:34:56] Foo: What?
";
        let c = Conversation::from_str(mock_for_frequency).unwrap();

        for (k, v) in c.frequency() {
            match k.as_str() {
                "00h" => assert_eq!(v, 1),
                "02h" => assert_eq!(v, 2),
                "03h" => assert_eq!(v, 1),
                "04h" => assert_eq!(v, 1),
                "05h" => assert_eq!(v, 1),
                "06h" => assert_eq!(v, 1),
                "23h" => assert_eq!(v, 1),
                _ => assert_eq!(v, 0),
            }
        }
    }

    macro_rules! assert_timeline_map_item {
        ($what: expr,$key: tt, $val: tt) => {
            let key = NaiveDateTime::parse_from_str($key, "%Y-%m-%dT%H:%M:%S").unwrap();
            assert!($what.contains_key(&key), "with key: {}", key);

            let val = $what.get(&key).unwrap().count();
            assert_eq!(val, $val, "with key: {}", key);
        };
    }

    #[test]
    fn timeline_map_daily_works() {
        let mock_for_daily = r"
[2001-01-19, 00:34:56] Kendrick: Sit down!
[2001-01-19, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-20, 00:34:56] Kendrick: Sit down!
[2001-01-21, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-22, 00:34:56] Kendrick: Sit down!
[2001-01-23, 23:59:59] Kendrick: Bitch, be humble.
    ";
        let c = Conversation::from_str(mock_for_daily).unwrap();
        let t = c.timeline_map(TimelineType::Daily);

        assert_eq!(t.len(), 5);

        assert_timeline_map_item!(t, "2001-01-19T00:00:00", 2);
        assert_timeline_map_item!(t, "2001-01-20T00:00:00", 1);
        assert_timeline_map_item!(t, "2001-01-21T00:00:00", 1);
        assert_timeline_map_item!(t, "2001-01-22T00:00:00", 1);
        assert_timeline_map_item!(t, "2001-01-23T00:00:00", 1);
    }
    #[test]
    fn timeline_map_weekly_works() {
        let mock_for_weekly = r"
[2001-01-01, 00:34:56] Kendrick: Sit down!
[2001-01-02, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-03, 00:34:56] Kendrick: Sit down!
[2001-01-04, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-05, 00:34:56] Kendrick: Sit down!
[2001-01-06, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-07, 00:34:56] Kendrick: Sit down, week's about to end.
[2001-01-08, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-09, 00:34:56] Kendrick: Sit down!
[2001-01-10, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-11, 00:34:56] Kendrick: Sit down!
[2001-01-12, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-13, 00:34:56] Kendrick: Sit down!
[2001-01-14, 23:59:59] Kendrick: Bitch, week's about to end.
[2001-01-15, 00:34:56] Kendrick: Sit down!
[2001-01-16, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-17, 00:34:56] Kendrick: Sit down!
[2001-01-18, 23:59:59] Kendrick: Bitch, be humble.";

        let c = Conversation::from_str(mock_for_weekly).unwrap();
        let t = c.timeline_map(TimelineType::Weekly);

        assert_eq!(t.keys().len(), 3);

        assert_timeline_map_item!(t, "2001-01-01T00:00:00", 7);
        assert_timeline_map_item!(t, "2001-01-08T00:00:00", 7);
        assert_timeline_map_item!(t, "2001-01-15T00:00:00", 4);
    }

    #[test]
    fn timeline_map_monthly_works() {
        let mock_for_monthly = r"
[2001-01-02, 00:34:56] Kendrick: Sit down!
[2001-01-02, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-03, 00:34:56] Kendrick: Sit down!
[2001-01-04, 23:59:59] Kendrick: Bitch, be humble.
[2001-01-05, 00:34:56] Kendrick: Sit down, last message of month.
[2001-02-06, 23:59:59] Kendrick: Bitch, be humble.
[2001-02-07, 00:34:56] Kendrick: Sit down!
[2001-02-08, 23:59:59] Kendrick: Bitch, be humble.
[2001-02-09, 00:34:56] Kendrick: Sit down, last message of month
[2001-03-10, 23:59:59] Kendrick: Bitch, be humble.
[2001-03-11, 00:34:56] Kendrick: Sit down!
[2001-03-12, 23:59:59] Kendrick: Bitch, be humble.
[2001-03-13, 00:34:56] Kendrick: Sit down!
[2001-03-14, 23:59:59] Kendrick: Bitch, last message of month.
[2001-07-15, 00:34:56] Kendrick: Sit down!
[2001-07-16, 23:59:59] Kendrick: Bitch, be humble.
[2001-07-17, 00:34:56] Kendrick: Sit down!
[2001-07-18, 23:59:59] Kendrick: Bitch, be humble.
";

        let c = Conversation::from_str(mock_for_monthly).unwrap();
        let t = c.timeline_map(TimelineType::Monthly);

        assert_eq!(t.keys().len(), 7);

        assert_timeline_map_item!(t, "2001-01-01T00:00:00", 5);
        assert_timeline_map_item!(t, "2001-02-01T00:00:00", 4);
        assert_timeline_map_item!(t, "2001-03-01T00:00:00", 5);
        assert_timeline_map_item!(t, "2001-04-01T00:00:00", 0);
        assert_timeline_map_item!(t, "2001-05-01T00:00:00", 0);
        assert_timeline_map_item!(t, "2001-06-01T00:00:00", 0);
        assert_timeline_map_item!(t, "2001-07-01T00:00:00", 4);
    }

    #[test]
    fn timeline_map_yearly_works() {
        let mock_for_yearly = r"`
[2001-02-13, 00:34:56] Kendrick: Sit down!
[2001-02-14, 10:34:56] Kendrick: Aye.
[2002-07-18, 23:59:59] Kendrick: Bitch, be humble!!
[2002-08-01, 13:59:59] Kendrick: Aye.
[2003-01-27, 00:34:56] Kendrick: Sit down!
[2004-12-01, 23:59:59] Kendrick: Bitch, be humble.
[2005-09-17, 00:34:56] Kendrick: Sit down!
[2006-07-18, 23:59:59] Kendrick: Bitch, be humble.
[2007-02-17, 00:34:56] Kendrick: Sit down!
[2008-01-06, 23:59:59] Kendrick: Bitch, be humble.
";
        let c = Conversation::from_str(mock_for_yearly).unwrap();
        let t = c.timeline_map(TimelineType::Yearly);

        assert_eq!(t.keys().len(), 8);

        assert_timeline_map_item!(t, "2001-01-01T00:00:00", 2);
        assert_timeline_map_item!(t, "2002-01-01T00:00:00", 2);
        assert_timeline_map_item!(t, "2003-01-01T00:00:00", 1);
        assert_timeline_map_item!(t, "2004-01-01T00:00:00", 1);
        assert_timeline_map_item!(t, "2005-01-01T00:00:00", 1);
        assert_timeline_map_item!(t, "2006-01-01T00:00:00", 1);
        assert_timeline_map_item!(t, "2007-01-01T00:00:00", 1);
        assert_timeline_map_item!(t, "2008-01-01T00:00:00", 1);
    }

    #[test]
    fn timeline_yearly_works() {
        let mock_for_yearly = r"`
[2001-02-13, 00:34:56] Kendrick: Sit down!
[2001-02-14, 10:34:56] Kendrick: Aye.
[2002-07-18, 23:59:59] Kendrick: Bitch, be humble!!
[2002-08-01, 13:59:59] Kendrick: Aye.
[2003-01-27, 00:34:56] Kendrick: Sit down!
[2004-12-01, 23:59:59] Kendrick: Bitch, be humble.
[2005-09-17, 00:34:56] Kendrick: Sit down!
[2006-07-18, 23:59:59] Kendrick: Bitch, be humble.
[2007-02-17, 00:34:56] Kendrick: Sit down!
[2008-01-06, 23:59:59] Kendrick: Bitch, be humble.
";
        let c = Conversation::from_str(mock_for_yearly).unwrap();
        let t = c.timeline(TimelineType::Yearly);

        let m: DateTimeHashMap<_> = t.clone().into();

        assert_eq!(m.len(), 8);
        // TODO: add more cases
    }
}
