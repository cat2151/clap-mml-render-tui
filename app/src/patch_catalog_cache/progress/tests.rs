use super::*;
use std::cell::RefCell;

#[test]
fn announces_work_before_calling_it_and_reports_its_result() {
    let events = RefCell::new(Vec::new());
    let result = run_with_report(
        "catalog scan",
        || {
            assert_eq!(&*events.borrow(), &["開始: catalog scan"]);
            events.borrow_mut().push("work".to_string());
            Ok(7)
        },
        |message| events.borrow_mut().push(message),
    )
    .unwrap();

    assert_eq!(result, 7);
    assert!(events.borrow()[2].starts_with("完了: catalog scan"));
}

#[test]
fn failed_work_is_reported_and_error_is_preserved() {
    let events = RefCell::new(Vec::new());
    let result = run_with_report::<()>(
        "catalog scan",
        || {
            assert_eq!(&*events.borrow(), &["開始: catalog scan"]);
            anyhow::bail!("source unavailable")
        },
        |message| events.borrow_mut().push(message),
    );

    assert_eq!(result.unwrap_err().to_string(), "source unavailable");
    assert!(events.borrow()[1].starts_with("失敗: catalog scan"));
    assert!(events.borrow()[1].contains("source unavailable"));
    assert_eq!(events.borrow().len(), 2);
}
