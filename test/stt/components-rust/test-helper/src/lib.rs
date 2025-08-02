#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::helper_exports::test_helper_api::*;
use std::cell::RefCell;

struct State {
    total: u64,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State { total: 0 });
}

struct Component;

impl Guest for Component {
    fn inc_and_get() -> u64 {
        STATE.with_borrow_mut(|state| {
            state.total += 1;
            state.total
        })
    }
}

bindings::export!(Component with_types_in bindings);


