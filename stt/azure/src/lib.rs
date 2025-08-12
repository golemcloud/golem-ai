mod client;
mod component;
mod conversions;

use component::Component;
use golem_stt::durability::{DurableStt, ExtendedGuest};

impl ExtendedGuest for Component {}

type DurableAzureComponent = DurableStt<Component>;

golem_stt::export_stt!(DurableAzureComponent with_types_in golem_stt);
