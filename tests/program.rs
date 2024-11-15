use common::{exec_integration, input, join};
use proc::java;
mod common;

#[test]
pub fn hello_world() {
    let input = input().with_std();
    let class = java!(r#"
        public class HelloWorld {
            public static void main(String[] args) {
                System.out.println("Hello, World");
            }
        }
    "#);

    exec_integration(input, class)
        .stdout(join(["Hello, World"]))
        .success();
}

#[test]
pub fn anonymous_classes() {
    let input = input().with_std();
    let class = java!(r#"
        public class AnonymousClasses {
            static abstract class MakeMeAnonymous {
                int x;
                abstract void work();
            }

            public static void main(String[] args) {
                MakeMeAnonymous an = new MakeMeAnonymous() {
                    int x = 10;
                    void work() {
                        System.out.println("Hello from anonymous");
                    }
                };

                an.work();
            }
        }
    "#);

    exec_integration(input, class)
        .stdout(join(["Hello from anonymous"]))
        .success();
}

#[test]
pub fn counting_arrays() {
    let input = input().with_std();
    let class = java!(r#"
        public class CountingArrays {
            public static void main(String[] args) {
                int[] a = {1, 3, 5};
                int loop1 = 0, loop2 = 0, loop3 = 0, counter = 0;

                counter++;

                outer:
                for (int i = 0; i < a.length; counter++, i++) {
                    loop1++;
                    counter++;
                    for (int j = 0; j < a.length; counter++, j++) {
                        loop2++;
                        counter++;
                        for (int k = 0; k < a.length; counter++, k++) {
                            loop3++;
                            counter++;
                            if (a[i] + a[j] == a[k]) {
                                break outer;
                            }
                        }
                    }
                }

                System.out.println(loop1);
                System.out.println(loop2);
                System.out.println(loop3);
                System.out.println(counter);
            }
        }
    "#);

    exec_integration(input, class)
        .stdout(join(["3", "9", "27", "79"]))
        .success();
}

#[test]
pub fn constructor_throws() {
    let input = input().with_std();
    let class = java!(r#"
        class Rational {
            int num;
            int dom;

            public Rational(int num, int dem) {
                if (dem == 0) {
                    throw new IllegalArgumentException("Denominator cannot be zero.");
                }
                this.num = num;
                this.dom = dem;
            }

            void print() {
                StringBuilder sb = new StringBuilder();
                sb
                  .append("num=")
                  .append(this.num)
                  .append(";dom=")
                  .append(this.dom);
                System.out.println(sb.toString());
            }
        }

        public class ThrowsConstructor {
            static void main0() {
                Rational r = new Rational(1,1);
                r.print();

                try {
                    r = new Rational(1, 0);
                    throw new IllegalStateException("impossible");
                } catch (IllegalArgumentException ex) {
                    System.out.println("caught IllegalArgumentException");
                }
            }

            public static void main(String[] args) {
                main0();
            }
        }
    "#);

    exec_integration(input, class)
        .stdout(join(["num=1;dom=1", "caught IllegalArgumentException"]))
        .success();
}

#[test]
pub fn linked_list() {
    let input = input().with_std();
    let class = java!(r#"
        import java.util.LinkedList;
        public class LinkedListTest {
            public static void main(String[] args) {
                LinkedList<String> ll = new LinkedList<String>();
                ll.add("hello world");

                System.out.println(ll.get(0));
            }
        }
    "#);

    exec_integration(input, class)
        .stdout(join(["hello world"]))
        .success();
}
