use async_lifecycle_fixture::{Phase, Runtime, Service};

#[test]
fn joining_after_stop_completes_the_managed_operation() {
    let runtime = Runtime;
    let service = Service::new();

    let task = service.start(&runtime);
    assert_eq!(service.phase(), Phase::Running);

    service.request_stop();
    runtime.join(task);

    assert_eq!(service.phase(), Phase::Stopped);
    assert_eq!(service.completed_operations(), 1);
}
