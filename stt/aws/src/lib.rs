mod client;
mod component;
mod conversions;
mod sigv4;
mod stream;

use component::Component;
use golem_stt::durability::{DurableStt, ExtendedGuest};

impl ExtendedGuest for Component {}

type DurableAwsComponent = DurableStt<Component>;

golem_stt::export_stt!(DurableAwsComponent with_types_in golem_stt);
