mod common;
use proc::java;
use common::{make_vm, load_test, attach_utils, execute_test, iassert_eq, sassert_eq, assert_null, assert_not_null};

#[test]
pub fn throw_index_oob() {
    let compiled = java!(
        r#"
        public class IndexOOB {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest0() {
                int[] arr = new int[0];
                try {
                    int outOfBounds = arr[1];
                } catch (ArrayIndexOutOfBoundsException ex) {
                    capture(ex.getMessage());
                }
            }

            static void runTest() {
                runTest0();
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("java/lang/ArrayIndexOutOfBoundsException: OOB @ 1", captures.next());
}

#[test]
pub fn throw_npe_internal() {
    let compiled = java!(
        r#"
        public class InternalNPE {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest0() {
                try {
                    Object o = null;
                    String impossible = o.toString();
                    capture("Didn't throw?");
                } catch (NullPointerException ex) {
                    capture(ex.getMessage());
                }
            }

            static void runTest() {
                runTest0();
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("java/lang/NullPointerException: NPE (cannot invoke method 'toString' on null)", captures.next());
}
