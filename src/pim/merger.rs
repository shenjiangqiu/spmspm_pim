use super::Component;

pub trait Merger {
    type SimContext;
}
impl<T> Component for T
where
    T: Merger,
{
    type SimContext = T::SimContext;

    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        todo!()
    }
}
