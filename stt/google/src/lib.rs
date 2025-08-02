mod client;
mod component;
mod conversions;
mod stream;

use component::Component;
use golem_stt::durability::{DurableStt, ExtendedGuest};

impl ExtendedGuest for Component {}

type DurableGoogleComponent = DurableStt<Component>;

golem_stt::export_stt!(DurableGoogleComponent with_types_in golem_stt);
