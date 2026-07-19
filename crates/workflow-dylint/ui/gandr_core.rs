#![crate_name = "gandr_core"]
#![allow(dead_code)]
#![allow(unconditional_recursion)]

mod checker
{
    struct Term;

    pub struct Rec;

    impl Rec
    {
        /// # Termination
        /// - reason: real checker recursion over input.
        /// - measure: remaining checked term structure.
        /// - boundedness: checked terms are finite.
        /// - input recursion: structural descent over the checked term.
        fn valid_non_none(
            &mut self,
            term: Term,
        )
        {
            self.valid_non_none(term);
        }

        /// # Termination
        /// - reason: false none claims are still rejected.
        /// - measure: remaining checked term structure.
        /// - boundedness: checked terms are finite.
        /// - input recursion: none.
        fn false_none(
            &mut self,
            term: Term,
        )
        {
            self.false_none(term);
        }
    }
}

mod mimic
{
    struct Term;

    pub struct Rec;

    impl Rec
    {
        /// # Termination
        /// - reason: similarly named receivers are not exempt.
        /// - measure: remaining fake term structure.
        /// - boundedness: fake terms are finite.
        /// - input recursion: structural descent over the fake term.
        fn wrong_non_none(
            &mut self,
            term: Term,
        )
        {
            self.wrong_non_none(term);
        }
    }
}

fn main()
{
}
