use golem_search::golem::search::types::IndexName;

use crate::client::{IndexItem, ListIndexesQueryParams};







pub fn to_list_indices_request(hits_per_page: Option<u16>, page: Option<u16>) -> ListIndexesQueryParams {
    ListIndexesQueryParams {
        hits_per_page,
        page,
    }
}


pub fn index_item_to_index_name(index_item: Vec<IndexItem>) -> Vec<IndexName> {
    index_item.into_iter().map(|item| IndexName::from(item.name)).collect()
}

