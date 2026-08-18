use crate::{AnyEntryId, FacetSpace, KeyTable};

#[derive(Clone, Copy)]
pub struct ColumnRegistryView<'a> {
    /// Column keys, sorted.
    pub facets: KeyTable<'a, FacetSpace>,
    /// `false` = dense
    pub sparse: &'a [bool],
    /// prefix offsets into the value space: column `i`
    /// owns values `col_runs[i]..col_runs[i+1]`
    pub col_runs: &'a [u32],
    /// Sparse entry ids, concatenated.
    /// Only columns with `sparse[i]` contribute
    pub ids: &'a [AnyEntryId],
    /// byte offsets into `bytes`
    pub bytes_offsets: &'a [u32],
    /// Value `v` is `bytes[bytes_offsets[v]..bytes_offsets[v+1]]`
    pub bytes: &'a [u8],
    /// Per column. Hash of the byte contract.
    /// `None` = not declared and not checked.
    pub layouts: &'a [Option<u64>],
}

#[derive(Clone, Copy)]
pub struct ColumnView<'a> {
    /// `None` = dense
    pub ids: Option<&'a [AnyEntryId]>,
    pub bytes_offsets: &'a [u32],
    pub bytes: &'a [u8],
    pub layout: Option<u64>,
}
