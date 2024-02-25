mod common;
use proc::java;
use common::{make_vm, load_test, attach_utils, execute_test, iassert_eq, sassert_eq, assert_null, assert_not_null};

#[test]
fn reference_arrays() {
    let compiled = java!(r#"
        public class ReferenceArrays {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest() {
                // Creating arrays with initialiser
                String[] referenceArray = new String[]{
                    "hello",
                    "world"
                };

                // Getting length
                capture(referenceArray.length);

                // Getting elements
                capture(referenceArray[0]);
                capture(referenceArray[1]);

                // Mutating array
                referenceArray[0] = "foo";
                referenceArray[1] = "bar";

                capture(referenceArray[0]);
                capture(referenceArray[1]);

                // Mutating array with empty strings
                referenceArray[0] = "";
                referenceArray[1] = "";

                capture(referenceArray[0]);
                capture(referenceArray[1]);

                // Mutating array with nulls
                referenceArray[0] = null;
                referenceArray[1] = null;

                capture(referenceArray[0]);
                capture(referenceArray[1]);
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    iassert_eq(2, captures.next());

    sassert_eq("hello", captures.next());
    sassert_eq("world", captures.next());

    sassert_eq("foo", captures.next());
    sassert_eq("bar", captures.next());

    sassert_eq("", captures.next());
    sassert_eq("", captures.next());

    assert_null(captures.next());
    assert_null(captures.next());
}

#[test]
fn local_variables() {
    let compiled = java!(r#"
        public class LocalVariables {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest() {
                // Including 5 to get the dynamic variant
                String s1 = "hello";
                String s2 = "world";
                String s3 = "foo";
                String s4 = "bar";
                String s5 = "baz";

                capture(s1);
                capture(s2);
                capture(s3);
                capture(s4);
                capture(s5);
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("hello", captures.next());
    sassert_eq("world", captures.next());
    sassert_eq("foo", captures.next());
    sassert_eq("bar", captures.next());
    sassert_eq("baz", captures.next());
}

#[test]
fn static_functions() {
    let compiled = java!(r#"
        public class StaticFunctions {
            static native void capture(int i);
            static native void capture(String s);

            static String returnString() {
                return "hello world";
            }

            static int returnInt() {
                return 1;
            }

            static void runTest() {
                capture(returnString());
                capture(returnInt());
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("hello world", captures.next());
    iassert_eq(1, captures.next());
}

#[test]
fn multidimensional_arrays() {
    let compiled = java!(r#"
        public class MultidimensionalArrays {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest() {
                int[][] arr = new int[3][3];

                // Make sure all edges are 0 init
                capture(arr[0][0]);
                capture(arr[0][2]);
                capture(arr[2][0]);
                capture(arr[2][2]);

                // Make sure we can write into it
                arr[1][1] = 30; 
                capture(arr[1][1]);

                // But that it doesn't corrupt anything
                capture(arr[0][0]);
                capture(arr[0][2]);
                capture(arr[2][0]);
                capture(arr[2][2]);

                // Make sure the lengths match up
                capture(arr.length);
                capture(arr[0].length);
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());

    iassert_eq(30, captures.next());

    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());
    iassert_eq(0, captures.next());

    iassert_eq(3, captures.next());
    iassert_eq(3, captures.next());
}

#[test]
fn throwing_exceptions() {
    let compiled = java!(r#"
        public class ThrowingExceptions {
            static native void capture(int i);
            static native void capture(String s);

            private static void throwException() {
                throw new IllegalStateException("die");
            }

            private static void nestedThrow() {
                throwException();
            }

            private static String thrownWithinMethod() {
                try {
                    throw new IllegalStateException("die");
                } catch (IllegalStateException e) {
                    return "Caught";
                }
            }

            private static String thrownOverMethod() {
                try {
                    throwException();
                    return "Not thrown";
                } catch (IllegalStateException e) {
                    return "Caught";
                }
            }

            private static String thrownOverManyMethods() {
                try {
                    nestedThrow();
                    return "Not thrown";
                } catch (IllegalStateException e) {
                    return "Caught";
                }
            }

            static void runTest0() {
                String status = thrownWithinMethod();
                capture(status);

                capture(thrownOverMethod());
                capture(thrownOverManyMethods());

                try {
                    status = "About to throw";
                    throw new IllegalStateException();
                } catch (IllegalStateException e) {
                    status = "Caught in method";
                }


                capture(status);

                // FIXME: We don't have the infra to assert exceptions thrown from tests yet.
                // Just checking that it properly ignores exceptions that aren't caught
                // throw new RuntimeException("thrown from main");
            }

            static void runTest() {
                // FIXME: Add this functionality
                // Move out of the entrypoint because we haven't setup reentry for test methods yet
                runTest0();
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("Caught", captures.next());
    sassert_eq("Caught", captures.next());
    sassert_eq("Caught", captures.next());
    sassert_eq("Caught in method", captures.next());
}

#[test]
fn cast_values() {
    let compiled = java!(r#"
        public class CastValues {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest0() {
                Object o = "string";
                String s = (String) o;
                capture(s);

                // We can re-erase it
                Object o2 = (Object) s;
                String s2 = (String) o2;
                capture(s2);

                // It doesn't mistake the component for the incoming type
                try {
                    String[] arr = (String[]) o;
                    throw new IllegalStateException("string[] cast");
                }
                catch (ClassCastException cce) {
                    capture("Caught String[] cce");
                }

                // It doesn't get confused with other object types
                try {
                    Class cls = (Class) o;
                    throw new IllegalStateException("class cast");
                }
                catch (ClassCastException cce) {
                    capture("Caught Class cce");
                }
                
                // It doesn't get confused with primitive arrays
                try {
                    byte[] arr = (byte[]) o;
                    throw new IllegalStateException("byte[] cast");
                }
                catch (ClassCastException cce) {
                    capture("Caught byte[] cce");
                }

                // It doesn't get confused with primitives
                try {
                    byte arr = (byte) o;
                    throw new IllegalStateException("primitive cast");
                }
                catch (ClassCastException cce) {
                    capture("Caught byte cce");
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

    sassert_eq("string", captures.next());
    sassert_eq("string", captures.next());

    sassert_eq("Caught String[] cce", captures.next());
    sassert_eq("Caught Class cce", captures.next());
    sassert_eq("Caught byte[] cce", captures.next());
    sassert_eq("Caught byte cce", captures.next());
}

#[test]
fn instance_of() {
    let compiled = java!(r#"
        public class InstanceOf {
            static native void capture(int i);
            static native void capture(String s);

            static void runTest() {
                Object o = "string";

                // It doesn't mistake the component for the incoming type
                if (o instanceof String[]) {
                    throw new IllegalStateException("array");
                }

                // It doesn't get confused with other object types
                if (o instanceof Class) {
                    throw new IllegalStateException("object");
                }

                // It doesn't get confused with primitive arrays
                if (o instanceof byte[]) {
                    throw new IllegalStateException("prim");
                }

                if (!(o instanceof String)) {
                    throw new IllegalStateException("self");
                }

                if (o instanceof String) {
                    capture("hit");
                }
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    sassert_eq("hit", captures.next());
}

#[test]
fn static_members() {
    let compiled = java!(r#"
        public class StaticMembers {
            static native void capture(int i);
            static native void capture(String s);

            static int initMember = 32;
            static int uninitMember;

            static void mutate() {
                initMember = 12;
            }

            static void runTest() {
                capture(initMember);

                capture(uninitMember);
                uninitMember = 10;
                capture(uninitMember);

                mutate();
                capture(initMember);
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    iassert_eq(32, captures.next());

    iassert_eq(0, captures.next());
    iassert_eq(10, captures.next());

    iassert_eq(12, captures.next());
}
#[test]
fn instance_members() {
    let compiled = java!(r#"
        public class InstanceMembers {
            static native void capture(int i);
            static native void capture(Object o);

            int intMember = 32;
            Object objectMember;

            void mutate() {
                intMember = 12;
            }

            static void runTest() {
                InstanceMembers s = new InstanceMembers();
                capture(s.intMember);

                capture(s.objectMember);
                s.objectMember = new Object();
                capture(s.objectMember);

                s.mutate();
                capture(s.intMember);
            }
        }"#
    );

    let vm = make_vm();
    let cls = load_test(&vm, compiled);
    let capture_id = attach_utils(cls.clone());
    let mut captures = execute_test(&vm, cls, capture_id);

    iassert_eq(32, captures.next());

    assert_null(captures.next());
    assert_not_null(captures.next());

    iassert_eq(12, captures.next());
}