use super::NaiveTime;
use crate::duration::Duration as OldDuration;
use crate::Timelike;

#[test]
fn test_time_from_hms_milli() {
    assert_eq!(
        NaiveTime::from_hms_milli_opt(3, 5, 7, 0),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 0).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_milli_opt(3, 5, 7, 777),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 777_000_000).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_milli_opt(3, 5, 7, 1_999),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 1_999_000_000).unwrap())
    );
    assert_eq!(NaiveTime::from_hms_milli_opt(3, 5, 7, 2_000), None);
    assert_eq!(NaiveTime::from_hms_milli_opt(3, 5, 7, 5_000), None); // overflow check
    assert_eq!(NaiveTime::from_hms_milli_opt(3, 5, 7, u32::MAX), None);
}

#[test]
fn test_time_from_hms_micro() {
    assert_eq!(
        NaiveTime::from_hms_micro_opt(3, 5, 7, 0),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 0).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_micro_opt(3, 5, 7, 333),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 333_000).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_micro_opt(3, 5, 7, 777_777),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 777_777_000).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_micro_opt(3, 5, 7, 1_999_999),
        Some(NaiveTime::from_hms_nano_opt(3, 5, 7, 1_999_999_000).unwrap())
    );
    assert_eq!(NaiveTime::from_hms_micro_opt(3, 5, 7, 2_000_000), None);
    assert_eq!(NaiveTime::from_hms_micro_opt(3, 5, 7, 5_000_000), None); // overflow check
    assert_eq!(NaiveTime::from_hms_micro_opt(3, 5, 7, u32::MAX), None);
}

#[test]
fn test_time_hms() {
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().hour(), 3);
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_hour(0),
        Some(NaiveTime::from_hms_opt(0, 5, 7).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_hour(23),
        Some(NaiveTime::from_hms_opt(23, 5, 7).unwrap())
    );
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_hour(24), None);
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_hour(u32::MAX), None);

    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().minute(), 5);
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_minute(0),
        Some(NaiveTime::from_hms_opt(3, 0, 7).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_minute(59),
        Some(NaiveTime::from_hms_opt(3, 59, 7).unwrap())
    );
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_minute(60), None);
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_minute(u32::MAX), None);

    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().second(), 7);
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_second(0),
        Some(NaiveTime::from_hms_opt(3, 5, 0).unwrap())
    );
    assert_eq!(
        NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_second(59),
        Some(NaiveTime::from_hms_opt(3, 5, 59).unwrap())
    );
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_second(60), None);
    assert_eq!(NaiveTime::from_hms_opt(3, 5, 7).unwrap().with_second(u32::MAX), None);
}

#[test]
fn test_time_add() {
    macro_rules! check {
        ($lhs:expr, $rhs:expr, $sum:expr) => {{
            assert_eq!($lhs + $rhs, $sum);
            //assert_eq!($rhs + $lhs, $sum);
        }};
    }

    let hmsm = |h, m, s, ms| NaiveTime::from_hms_milli_opt(h, m, s, ms).unwrap();

    check!(hmsm(3, 5, 7, 900), OldDuration::zero(), hmsm(3, 5, 7, 900));
    check!(hmsm(3, 5, 7, 900), OldDuration::milliseconds(100), hmsm(3, 5, 8, 0));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(-1800), hmsm(3, 5, 6, 500));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(-800), hmsm(3, 5, 7, 500));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(-100), hmsm(3, 5, 7, 1_200));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(100), hmsm(3, 5, 7, 1_400));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(800), hmsm(3, 5, 8, 100));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::milliseconds(1800), hmsm(3, 5, 9, 100));
    check!(hmsm(3, 5, 7, 900), OldDuration::seconds(86399), hmsm(3, 5, 6, 900)); // overwrap
    check!(hmsm(3, 5, 7, 900), OldDuration::seconds(-86399), hmsm(3, 5, 8, 900));
    check!(hmsm(3, 5, 7, 900), OldDuration::days(12345), hmsm(3, 5, 7, 900));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::days(1), hmsm(3, 5, 7, 300));
    check!(hmsm(3, 5, 7, 1_300), OldDuration::days(-1), hmsm(3, 5, 8, 300));

    // regression tests for #37
    check!(hmsm(0, 0, 0, 0), OldDuration::milliseconds(-990), hmsm(23, 59, 59, 10));
    check!(hmsm(0, 0, 0, 0), OldDuration::milliseconds(-9990), hmsm(23, 59, 50, 10));
}

#[test]
fn test_time_overflowing_add() {
    let hmsm = |h, m, s, ms| NaiveTime::from_hms_milli_opt(h, m, s, ms).unwrap();

    assert_eq!(
        hmsm(3, 4, 5, 678).overflowing_add_signed(OldDuration::hours(11)),
        (hmsm(14, 4, 5, 678), 0)
    );
    assert_eq!(
        hmsm(3, 4, 5, 678).overflowing_add_signed(OldDuration::hours(23)),
        (hmsm(2, 4, 5, 678), 86_400)
    );
    assert_eq!(
        hmsm(3, 4, 5, 678).overflowing_add_signed(OldDuration::hours(-7)),
        (hmsm(20, 4, 5, 678), -86_400)
    );

    // overflowing_add_signed with leap seconds may be counter-intuitive
    assert_eq!(
        hmsm(3, 4, 5, 1_678).overflowing_add_signed(OldDuration::days(1)),
        (hmsm(3, 4, 5, 678), 86_400)
    );
    assert_eq!(
        hmsm(3, 4, 5, 1_678).overflowing_add_signed(OldDuration::days(-1)),
        (hmsm(3, 4, 6, 678), -86_400)
    );
}

#[test]
fn test_time_addassignment() {
    let hms = |h, m, s| NaiveTime::from_hms_opt(h, m, s).unwrap();
    let mut time = hms(12, 12, 12);
    time += OldDuration::hours(10);
    assert_eq!(time, hms(22, 12, 12));
    time += OldDuration::hours(10);
    assert_eq!(time, hms(8, 12, 12));
}

#[test]
fn test_time_subassignment() {
    let hms = |h, m, s| NaiveTime::from_hms_opt(h, m, s).unwrap();
    let mut time = hms(12, 12, 12);
    time -= OldDuration::hours(10);
    assert_eq!(time, hms(2, 12, 12));
    time -= OldDuration::hours(10);
    assert_eq!(time, hms(16, 12, 12));
}

#[test]
fn test_time_sub() {
    macro_rules! check {
        ($lhs:expr, $rhs:expr, $diff:expr) => {{
            // `time1 - time2 = duration` is equivalent to `time2 - time1 = -duration`
            assert_eq!($lhs.signed_duration_since($rhs), $diff);
            assert_eq!($rhs.signed_duration_since($lhs), -$diff);
        }};
    }

    let hmsm = |h, m, s, ms| NaiveTime::from_hms_milli_opt(h, m, s, ms).unwrap();

    check!(hmsm(3, 5, 7, 900), hmsm(3, 5, 7, 900), OldDuration::zero());
    check!(hmsm(3, 5, 7, 900), hmsm(3, 5, 7, 600), OldDuration::milliseconds(300));
    check!(hmsm(3, 5, 7, 200), hmsm(2, 4, 6, 200), OldDuration::seconds(3600 + 60 + 1));
    check!(
        hmsm(3, 5, 7, 200),
        hmsm(2, 4, 6, 300),
        OldDuration::seconds(3600 + 60) + OldDuration::milliseconds(900)
    );

    // treats the leap second as if it coincides with the prior non-leap second,
    // as required by `time1 - time2 = duration` and `time2 - time1 = -duration` equivalence.
    check!(hmsm(3, 5, 7, 200), hmsm(3, 5, 6, 1_800), OldDuration::milliseconds(400));
    check!(hmsm(3, 5, 7, 1_200), hmsm(3, 5, 6, 1_800), OldDuration::milliseconds(1400));
    check!(hmsm(3, 5, 7, 1_200), hmsm(3, 5, 6, 800), OldDuration::milliseconds(1400));

    // additional equality: `time1 + duration = time2` is equivalent to
    // `time2 - time1 = duration` IF AND ONLY IF `time2` represents a non-leap second.
    assert_eq!(hmsm(3, 5, 6, 800) + OldDuration::milliseconds(400), hmsm(3, 5, 7, 200));
    assert_eq!(hmsm(3, 5, 6, 1_800) + OldDuration::milliseconds(400), hmsm(3, 5, 7, 200));
}

#[test]
fn test_core_duration_ops() {
    use core::time::Duration;

    let mut t = NaiveTime::from_hms_opt(11, 34, 23).unwrap();
    let same = t + Duration::ZERO;
    assert_eq!(t, same);

    t += Duration::new(3600, 0);
    assert_eq!(t, NaiveTime::from_hms_opt(12, 34, 23).unwrap());

    t -= Duration::new(7200, 0);
    assert_eq!(t, NaiveTime::from_hms_opt(10, 34, 23).unwrap());
}

#[test]
fn test_time_fmt() {
    assert_eq!(
        format!("{}", NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap()),
        "23:59:59.999"
    );
    assert_eq!(
        format!("{}", NaiveTime::from_hms_milli_opt(23, 59, 59, 1_000).unwrap()),
        "23:59:60"
    );
    assert_eq!(
        format!("{}", NaiveTime::from_hms_milli_opt(23, 59, 59, 1_001).unwrap()),
        "23:59:60.001"
    );
    assert_eq!(
        format!("{}", NaiveTime::from_hms_micro_opt(0, 0, 0, 43210).unwrap()),
        "00:00:00.043210"
    );
    assert_eq!(
        format!("{}", NaiveTime::from_hms_nano_opt(0, 0, 0, 6543210).unwrap()),
        "00:00:00.006543210"
    );

    // the format specifier should have no effect on `NaiveTime`
    assert_eq!(
        format!("{:30}", NaiveTime::from_hms_milli_opt(3, 5, 7, 9).unwrap()),
        "03:05:07.009"
    );
}

#[test]
fn test_time_from_str() {
    // valid cases
    let valid = [
        "0:0:0",
        "0:0:0.0000000",
        "0:0:0.0000003",
        " 4 : 3 : 2.1 ",
        " 09:08:07 ",
        " 09:08 ",
        " 9:8:07 ",
        "01:02:03",
        "4:3:2.1",
        "9:8:7",
        "09:8:7",
        "9:08:7",
        "9:8:07",
        "09:08:7",
        "09:8:07",
        "09:08:7",
        "9:08:07",
        "09:08:07",
        "9:8:07.123",
        "9:08:7.123",
        "09:8:7.123",
        "09:08:7.123",
        "9:08:07.123",
        "09:8:07.123",
        "09:08:07.123",
        "09:08:07.123",
        "09:08:07.1234",
        "09:08:07.12345",
        "09:08:07.123456",
        "09:08:07.1234567",
        "09:08:07.12345678",
        "09:08:07.123456789",
        "09:08:07.1234567891",
        "09:08:07.12345678912",
        "23:59:60.373929310237",
    ];
    for &s in &valid {
        eprintln!("test_time_parse_from_str valid {:?}", s);
        let d = match s.parse::<NaiveTime>() {
            Ok(d) => d,
            Err(e) => panic!("parsing `{}` has failed: {}", s, e),
        };
        let s_ = format!("{:?}", d);
        // `s` and `s_` may differ, but `s.parse()` and `s_.parse()` must be same
        let d_ = match s_.parse::<NaiveTime>() {
            Ok(d) => d,
            Err(e) => {
                panic!("`{}` is parsed into `{:?}`, but reparsing that has failed: {}", s, d, e)
            }
        };
        assert!(
            d == d_,
            "`{}` is parsed into `{:?}`, but reparsed result \
                              `{:?}` does not match",
            s,
            d,
            d_
        );
    }

    // some invalid cases
    // since `ParseErrorKind` is private, all we can do is to check if there was an error
    let invalid = [
        "",                  // empty
        "x",                 // invalid
        "15",                // missing data
        "15:8:",             // trailing colon
        "15:8:x",            // invalid data
        "15:8:9x",           // invalid data
        "23:59:61",          // invalid second (out of bounds)
        "23:54:35 GMT",      // invalid (timezone non-sensical for NaiveTime)
        "23:54:35 +0000",    // invalid (timezone non-sensical for NaiveTime)
        "1441497364.649",    // valid datetime, not a NaiveTime
        "+1441497364.649",   // valid datetime, not a NaiveTime
        "+1441497364",       // valid datetime, not a NaiveTime
        "001:02:03",         // invalid hour
        "01:002:03",         // invalid minute
        "01:02:003",         // invalid second
        "12:34:56.x",        // invalid fraction
        "12:34:56. 0",       // invalid fraction format
        "09:08:00000000007", // invalid second / invalid fraction format
    ];
    for &s in &invalid {
        eprintln!("test_time_parse_from_str invalid {:?}", s);
        assert!(s.parse::<NaiveTime>().is_err());
    }
}

#[test]
fn test_time_parse_from_str() {
    let hms = |h, m, s| NaiveTime::from_hms_opt(h, m, s).unwrap();
    assert_eq!(
        NaiveTime::parse_from_str("2014-5-7T12:34:56+09:30", "%Y-%m-%dT%H:%M:%S%z"),
        Ok(hms(12, 34, 56))
    ); // ignore date and offset
    assert_eq!(NaiveTime::parse_from_str("PM 12:59", "%P %H:%M"), Ok(hms(12, 59, 0)));
    assert_eq!(NaiveTime::parse_from_str("12:59 \n\t PM", "%H:%M \n\t %P"), Ok(hms(12, 59, 0)));
    assert_eq!(NaiveTime::parse_from_str("\t\t12:59\tPM\t", "\t\t%H:%M\t%P\t"), Ok(hms(12, 59, 0)));
    assert_eq!(
        NaiveTime::parse_from_str("\t\t1259\t\tPM\t", "\t\t%H%M\t\t%P\t"),
        Ok(hms(12, 59, 0))
    );
    assert!(NaiveTime::parse_from_str("12:59 PM", "%H:%M\t%P").is_ok());
    assert!(NaiveTime::parse_from_str("\t\t12:59 PM\t", "\t\t%H:%M\t%P\t").is_ok());
    assert!(NaiveTime::parse_from_str("12:59  PM", "%H:%M %P").is_ok());
    assert!(NaiveTime::parse_from_str("12:3456", "%H:%M:%S").is_err());
}
