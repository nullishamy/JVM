mod common;

use proc::java;
use common::{make_vm, load_test, attach_utils, execute_test, iassert_eq, dassert_eq};

#[test]
fn simple_capture() {
    let compiled = java!("
        public class SimpleCapture {
            static native void capture(int i);

            static void runTest() {
                capture(1);
            }
        }"
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let captures = execute_test(&vm, cls, capture_id);

    iassert_eq(1, captures.get(0));
}

#[test]
fn many_capture() {
    let compiled = java!("
        public class ManyCapture {
            static native void capture(int i);

            static void runTest() {
                capture(1);
                capture(2);
                capture(1);
            }
        }"
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let captures = execute_test(&vm, cls, capture_id);

    iassert_eq(1, captures.get(0));
    iassert_eq(2, captures.get(1));
    iassert_eq(1, captures.get(2));
}

#[test]
fn capture_doubles() {
    let compiled = java!("
        public class CaptureDoubles {
            static native void capture(double d);

            static void runTest() {
                capture(1);
                capture(2);
                capture(1);
            }
        }"
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let captures = execute_test(&vm, cls, capture_id);

    dassert_eq(1.0, captures.get(0));
    dassert_eq(2.0, captures.get(1));
    dassert_eq(1.0, captures.get(2));
}