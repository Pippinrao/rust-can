/// CAN message filter definitions.
///
/// Filters can be applied in hardware (if supported by the adapter) or
/// in software (fallback). Each filter consists of a CAN ID, a mask,
/// and an optional extended flag.
///
/// The filter matches a message when:
/// ```text
/// (received_can_id & can_mask) == (can_id & can_mask)
/// ```
/// and (if `extended` is set) `received_is_extended == extended`.
use serde::{Deserialize, Serialize};

use crate::message::CanMessage;

/// A single CAN message filter.
///
/// # Examples
///
/// ```
/// use rust_can_core::filter::CanFilter;
///
/// // Match only CAN ID 0x123 (standard)
/// let filter = CanFilter::new(0x123, 0x7FF, Some(false));
///
/// // Match all extended IDs starting with 0x18
/// let filter = CanFilter::new(0x18000000, 0x1F000000, Some(true));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanFilter {
    /// The CAN ID to match against.
    pub can_id: u32,
    /// Mask applied to both the filter ID and the received ID.
    /// Only bits where the mask is 1 are compared.
    pub can_mask: u32,
    /// If `Some(true)`, only match extended frames.
    /// If `Some(false)`, only match standard frames.
    /// If `None`, match both.
    pub extended: Option<bool>,
}

impl CanFilter {
    /// Create a new filter.
    pub fn new(can_id: u32, can_mask: u32, extended: Option<bool>) -> Self {
        Self {
            can_id,
            can_mask,
            extended,
        }
    }

    /// Check if a message matches this filter.
    ///
    /// Returns `true` if the message passes the filter.
    pub fn matches(&self, msg: &CanMessage) -> bool {
        // Check extended/standard match
        if let Some(filter_extended) = self.extended
            && filter_extended != msg.is_extended_id()
        {
            return false;
        }

        // Check ID match with mask
        (self.can_id ^ msg.arbitration_id) & self.can_mask == 0
    }
}

/// A collection of CAN filters.
///
/// A message matches if it passes **any** of the filters (OR semantics).
/// An empty filter set matches **all** messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanFilters {
    filters: Vec<CanFilter>,
}

impl CanFilters {
    /// Create an empty filter set (matches all messages).
    pub fn new() -> Self {
        Self { filters: Vec::new() }
    }

    /// Create from a list of filters.
    pub fn from_filters(filters: Vec<CanFilter>) -> Self {
        Self { filters }
    }

    /// Add a filter.
    pub fn add(&mut self, filter: CanFilter) {
        self.filters.push(filter);
    }

    /// Check if a message matches any filter.
    ///
    /// Returns `true` if the filter set is empty (match-all) or
    /// if the message matches at least one filter.
    pub fn matches(&self, msg: &CanMessage) -> bool {
        if self.filters.is_empty() {
            return true;
        }
        self.filters.iter().any(|f| f.matches(msg))
    }

    /// Returns the number of filters.
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Returns `true` if there are no filters (matches all).
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Get a reference to the filter list (for hardware filter programming).
    pub fn as_slice(&self) -> &[CanFilter] {
        &self.filters
    }
}

impl From<Vec<CanFilter>> for CanFilters {
    fn from(filters: Vec<CanFilter>) -> Self {
        Self { filters }
    }
}

impl From<CanFilter> for CanFilters {
    fn from(filter: CanFilter) -> Self {
        Self {
            filters: vec![filter],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CanMessage;

    #[test]
    fn test_filter_exact_match() {
        let filter = CanFilter::new(0x123, 0x7FF, Some(false));
        let msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        assert!(filter.matches(&msg));
    }

    #[test]
    fn test_filter_no_match() {
        let filter = CanFilter::new(0x123, 0x7FF, Some(false));
        let msg = CanMessage::new(0x456, &[0x01], false).unwrap();
        assert!(!filter.matches(&msg));
    }

    #[test]
    fn test_filter_with_mask() {
        // Match all IDs from 0x100 to 0x1FF
        let filter = CanFilter::new(0x100, 0x700, Some(false));
        let msg1 = CanMessage::new(0x123, &[0x01], false).unwrap();
        let msg2 = CanMessage::new(0x2FF, &[0x01], false).unwrap();
        assert!(filter.matches(&msg1));
        assert!(!filter.matches(&msg2));
    }

    #[test]
    fn test_filter_extended_check() {
        let filter = CanFilter::new(0x123, 0x7FF, Some(true));
        let std_msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        let ext_msg = CanMessage::new(0x123, &[0x01], true).unwrap();
        assert!(!filter.matches(&std_msg));
        assert!(filter.matches(&ext_msg));
    }

    #[test]
    fn test_empty_filters_matches_all() {
        let filters = CanFilters::new();
        let msg = CanMessage::new(0x456, &[0x01], false).unwrap();
        assert!(filters.matches(&msg));
    }

    #[test]
    fn test_filters_or_semantics() {
        let filters = CanFilters::from_filters(vec![
            CanFilter::new(0x100, 0x7FF, Some(false)),
            CanFilter::new(0x200, 0x7FF, Some(false)),
        ]);
        let msg = CanMessage::new(0x200, &[0x01], false).unwrap();
        assert!(filters.matches(&msg));
        let msg2 = CanMessage::new(0x300, &[0x01], false).unwrap();
        assert!(!filters.matches(&msg2));
    }

    #[test]
    fn test_filter_extended_none_matches_both_frame_types() {
        let filter = CanFilter::new(0x123, 0x7FF, None);
        let std_msg = CanMessage::new(0x123, &[0x01], false).unwrap();
        let ext_msg = CanMessage::new(0x123, &[0x01], true).unwrap();
        assert!(filter.matches(&std_msg));
        assert!(filter.matches(&ext_msg));
    }

    #[test]
    fn test_filters_collection_api_and_from_conversions() {
        let mut filters = CanFilters::new();
        assert!(filters.is_empty());
        assert_eq!(filters.len(), 0);

        let filter = CanFilter::new(0x100, 0x700, Some(false));
        filters.add(filter);
        assert_eq!(filters.len(), 1);
        assert!(!filters.is_empty());
        assert_eq!(filters.as_slice().len(), 1);

        let from_single: CanFilters = CanFilter::new(0x200, 0x7FF, None).into();
        assert_eq!(from_single.len(), 1);
        assert!(from_single.matches(&CanMessage::new(0x200, &[0x01], false).unwrap()));

        let from_vec: CanFilters = vec![
            CanFilter::new(0x300, 0x7FF, Some(false)),
            CanFilter::new(0x400, 0x7FF, Some(false)),
        ]
        .into();
        assert_eq!(from_vec.len(), 2);
        assert!(from_vec.matches(&CanMessage::new(0x400, &[0x01], false).unwrap()));
    }

    #[test]
    fn test_filter_standard_only_rejects_extended_id() {
        let filter = CanFilter::new(0x18FF_0000, 0x1FFF_0000, Some(false));
        let ext_msg = CanMessage::new(0x18FF_50E5, &[0x01], true).unwrap();
        assert!(!filter.matches(&ext_msg));
    }
}
