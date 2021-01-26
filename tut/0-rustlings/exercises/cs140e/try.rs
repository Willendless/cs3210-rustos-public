// FIXME: Make me compile. Diff budget: 12 line additions and 2 characters.

struct ErrorA;
struct ErrorB;

enum Error {
    A(ErrorA),
    B(ErrorB)
}

<<<<<<< HEAD
impl From<ErrorB> for Error {
    fn from(err: ErrorB) -> Self {
        Error::B(err)
    }
}

impl From<ErrorA> for Error {
    fn from(err: ErrorA) -> Self {
        Error::A(err)
    }
}

=======
>>>>>>> skeleton/lab2
fn do_a() -> Result<u16, ErrorA> {
    Err(ErrorA)
}

fn do_b() -> Result<u32, ErrorB> {
    Err(ErrorB)
}

fn do_both() -> Result<(u16, u32), Error> {
    Ok((do_a()?, do_b()?))
}

fn main() { }
