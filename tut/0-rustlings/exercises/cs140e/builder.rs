// FIXME: Make me pass! Diff budget: 30 lines.

<<<<<<< HEAD
#[derive(Default)]
=======
// I AM NOT DONE

>>>>>>> skeleton/lab2
struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

<<<<<<< HEAD
impl Builder {
    // fn string(...
    fn string(&mut self, s: &str) -> &mut Self {
        self.string = Some(String::from(s));
        self
    }

    // fn number(...
    fn number(&mut self, num: usize) -> &mut Self {
        self.number = Some(num);
        self
    }
}

impl ToString for Builder {
    fn to_string(&self) -> String {
        let mut a = String::from("");
        let mut b = String::from("");
        if let Some(s) = &self.string {
            a = String::from(s);
        }
        if let Some(num) = self.number {
            b = num.to_string();
        }
        if &a == "" {
            b
        } else if &b == "" {
            a
        } else {
            a + " " + &b
        }
    }
}

=======
>>>>>>> skeleton/lab2
// Do not modify this function.
#[test]
fn builder() {
    let empty = Builder::default().to_string();
    assert_eq!(empty, "");

    let just_str = Builder::default().string("hi").to_string();
    assert_eq!(just_str, "hi");

    let just_num = Builder::default().number(254).to_string();
    assert_eq!(just_num, "254");

    let a = Builder::default()
        .string("hello, world!")
        .number(200)
        .to_string();

    assert_eq!(a, "hello, world! 200");

    let b = Builder::default()
        .string("hello, world!")
        .number(200)
        .string("bye now!")
        .to_string();

    assert_eq!(b, "bye now! 200");

<<<<<<< HEAD
    let c = Builder::default().string(&"heap!".to_owned()).to_string();
=======
    let c = Builder::default()
        .string("heap!".to_owned())
        .to_string();
>>>>>>> skeleton/lab2

    assert_eq!(c, "heap!");
}
