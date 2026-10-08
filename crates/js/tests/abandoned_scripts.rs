//! Scripts that outrun their time limit (its own process: the cap on abandoned scripts is global).

use std::time::{Duration, Instant};

use pdfcraft_js::{DocInfo, Event, Limits, Outcome, run_within};

/// A loop calling a function that loops: every frame stays under the engine's loop limit, so
/// only the time limit stops it. It does end, a few seconds later, freeing its thread.
const SLOW: &str =
    "function f() { var n = 0; for (var j = 0; j < 900000; j++) n++; return n; } var t = 0; for (var i = 0; i < 100; i++) t += f(); event.value = t;";

fn go(script: &str, timeout: Duration) -> Outcome {
    run_within(script, &Event::field("Calculate", "total", "7"), &DocInfo::default(), &[], &[], Limits::default(), timeout)
}

#[test]
fn slow_scripts_are_abandoned_and_capped_until_they_finish() {
    let started = Instant::now();
    let o = go(SLOW, Duration::from_millis(100));
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
    assert!(o.error.as_deref().is_some_and(|e| e.contains("abandoned")), "{o:?}");
    assert!(o.rc && o.value == "7", "the value is left alone: {o:?}");
    // Three more fill the cap: the next script doesn't start.
    for _ in 0..3 {
        assert!(go(SLOW, Duration::from_millis(100)).error.is_some_and(|e| e.contains("abandoned")));
    }
    let refused = go("event.value = 1;", Duration::from_secs(5));
    assert!(refused.error.as_deref().is_some_and(|e| e.contains("still running")), "{refused:?}");
    // Once the abandoned scripts end, scripts run again.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let o = go("event.value = 1;", Duration::from_secs(5));
        if o.error.is_none() {
            assert_eq!(o.value, "1");
            break;
        }
        assert!(Instant::now() < deadline, "the abandoned scripts never released their slots: {o:?}");
        std::thread::sleep(Duration::from_millis(200));
    }
}
