use core::any::type_name;

use alloc::vec::Vec;

use crate::{
    Id, Property, StateId,
    facet::{Facet, Storage},
    name::Name,
    registry::{BuildErrorKind, RegistryBuilder, RegistryError},
};

#[derive(Debug)]
struct Block;

#[derive(Debug, PartialEq)]
struct Hardness(u32);
impl Facet for Hardness {}

#[derive(Debug, PartialEq)]
struct Model(u32);
impl Facet for Model {
    const STORAGE: Storage<Self> = Storage::Required;
}

#[derive(Debug, PartialEq)]
struct Solid(bool);
impl Facet for Solid {
    const STORAGE: Storage<Self> = Storage::Dense(|| Solid(false));
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Facing {
    North,
    East,
    South,
    West,
}
impl Property for Facing {
    const COUNT: u32 = 4;
    fn to_raw(self) -> u32 {
        self as u32
    }
    fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::North,
            1 => Self::East,
            2 => Self::South,
            _ => Self::West,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Lit(bool);
impl Property for Lit {
    const COUNT: u32 = 2;
    fn to_raw(self) -> u32 {
        u32::from(self.0)
    }
    fn from_raw(raw: u32) -> Self {
        Self(raw == 1)
    }
}

struct Huge(u32); // for overflow tests
impl Property for Huge {
    const COUNT: u32 = u32::MAX;
    fn to_raw(self) -> u32 {
        self.0
    }
    fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

fn n(s: &str) -> Name {
    s.parse().unwrap()
}

#[test]
fn ids_follow_name_order() {
    let mut b = RegistryBuilder::<Block, Name>::new();
    b.create(n("t:c")).unwrap();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    assert_eq!(r.id("t:a").unwrap().raw(), 0);
    assert_eq!(r.id("t:b").unwrap().raw(), 1);
    assert_eq!(r.id("t:c").unwrap().raw(), 2);
}

#[test]
fn duplicate_name() {
    let mut b = RegistryBuilder::<Block, Name>::new();
    b.create(n("t:a")).unwrap();
    assert_eq!(
        b.create(n("t:a")).err(),
        Some(RegistryError::DuplicateStableId(n("t:a")))
    )
}

#[test]
fn value_follows_entry_through_the_sort() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().with(Hardness(5));
    b.create(n("t:a")).unwrap();
    let r = b.build().unwrap();
    let col = r.column::<Hardness>().unwrap();
    let c = r.id("t:c").unwrap();
    let a = r.id("t:a").unwrap();
    assert_eq!(col.get(c), Some(&Hardness(5)));
    assert_eq!(col.get(a), None);
}

#[test]
fn dense_fills_missing_entries() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    let col = r.column::<Solid>().unwrap();
    assert_eq!(col[r.id("t:a").unwrap()], Solid(true));
    assert_eq!(col[r.id("t:b").unwrap()], Solid(false));
}

#[test]
fn remap_reports_missing() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    let remap = r.remap([n("t:b"), n("t:x"), n("t:a")].iter());
    assert_eq!(remap.slots, [r.id("t:b"), None, r.id("t:a")]);
    assert_eq!(remap.missing, [(1, n("t:x"))]);
}

#[test]
fn column_on_never_added() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    let hardness_column = r.column::<Hardness>();
    assert!(hardness_column.is_none())
}

#[test]
fn get_on_never_added() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    let solid_column = r.column::<Solid>().unwrap();
    let something_solid = solid_column.get(Id::from_raw(100));
    assert!(something_solid.is_none())
}

#[test]
fn get_id_on_unknown_name() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    let c = r.id("t:c");
    assert!(c.is_none())
}

#[test]
fn get_unknown_name() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    assert!(b.entry(n("t:c")).get().is_none());
    assert!(b.build().unwrap().column::<Hardness>().is_none());
}

#[test]
fn get_known_name() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    b.entry(n("t:a")).get().unwrap().with(Hardness(15));
    let r = b.build().unwrap();
    let a = r.id("t:a").unwrap();
    assert_eq!(r.column::<Hardness>().unwrap().get(a).unwrap().0, 15);
}

#[test]
fn missing_required() {
    let mut b = RegistryBuilder::<Block>::new();
    b.declare::<Model>();
    b.create(n("t:a")).unwrap().with(Solid(true));
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![
            BuildErrorKind::MissingComponent {
                entry: n("t:a"),
                component: type_name::<Model>()
            },
            BuildErrorKind::MissingComponent {
                entry: n("t:b"),
                component: type_name::<Model>()
            }
        ]
    )
}

#[test]
fn duplicate_facet() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a"))
        .unwrap()
        .with(Solid(true))
        .with(Solid(false));
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::DuplicateComponent {
            entry: n("t:a"),
            component: type_name::<Solid>()
        }]
    );
}

#[test]
fn require_present_is_not_reported() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap().with(Model(1));
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::MissingComponent {
            entry: n("t:a"),
            component: type_name::<Model>()
        }]
    )
}

#[test]
fn duplicate_does_not_fake_missing() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().with(Model(1)).with(Model(2));
    b.create(n("t:b")).unwrap().with(Model(3));
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::DuplicateComponent {
            entry: n("t:a"),
            component: type_name::<Model>()
        }]
    )
}

#[test]
fn triple_attach_reports_two_duplicates() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a"))
        .unwrap()
        .with(Model(1))
        .with(Model(2))
        .with(Model(3));
    assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2)
}

#[test]
fn pin() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().pin(Id::from_raw(0));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    assert_eq!(r.id("t:c").unwrap().raw(), 0);
    assert_eq!(r.id("t:a").unwrap().raw(), 1);
    assert_eq!(r.id("t:b").unwrap().raw(), 2);
}

#[test]
fn pinned_in_order() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().pin(Id::from_raw(2));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    assert_eq!(r.id("t:a").unwrap().raw(), 0);
    assert_eq!(r.id("t:b").unwrap().raw(), 1);
    assert_eq!(r.id("t:c").unwrap().raw(), 2);
}

#[test]
fn pinned_in_middle() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().pin(Id::from_raw(1));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let r = b.build().unwrap();
    assert_eq!(r.id("t:a").unwrap().raw(), 0);
    assert_eq!(r.id("t:c").unwrap().raw(), 1);
    assert_eq!(r.id("t:b").unwrap().raw(), 2);
}

#[test]
fn pinned_out_of_range() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().pin(Id::from_raw(3));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::PinOutOfRange {
            entry: n("t:c"),
            slot: 3,
            len: 3
        }]
    );
}

#[test]
fn pin_conflict() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().pin(Id::from_raw(0));
    b.create(n("t:a")).unwrap().pin(Id::from_raw(0));
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::PinConflict {
            first: n("t:c"),
            second: n("t:a"),
            slot: 0,
        }]
    );
}

#[test]
fn triple_pinned() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c"))
        .unwrap()
        .pin(Id::from_raw(1))
        .pin(Id::from_raw(2))
        .pin(Id::from_raw(9));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![
            BuildErrorKind::DoublePin {
                entry: n("t:c"),
                first: 1,
                second: 2
            },
            BuildErrorKind::DoublePin {
                entry: n("t:c"),
                first: 1,
                second: 9
            }
        ]
    );
}

#[test]
fn double_pinned() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c"))
        .unwrap()
        .pin(Id::from_raw(1))
        .pin(Id::from_raw(9));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::DoublePin {
            entry: n("t:c"),
            first: 1,
            second: 9
        }]
    );
}

#[test]
fn double_pinned_with_same_id() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c"))
        .unwrap()
        .pin(Id::from_raw(1))
        .pin(Id::from_raw(1));
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::DoublePin {
            entry: n("t:c"),
            first: 1,
            second: 1
        }]
    );
}

#[test]
fn double_pinned_twice() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c"))
        .unwrap()
        .pin(Id::from_raw(1))
        .pin(Id::from_raw(9));
    b.create(n("t:a"))
        .unwrap()
        .pin(Id::from_raw(2))
        .pin(Id::from_raw(8));
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![
            BuildErrorKind::DoublePin {
                entry: n("t:c"),
                first: 1,
                second: 9
            },
            BuildErrorKind::DoublePin {
                entry: n("t:a"),
                first: 2,
                second: 8
            }
        ]
    );
}

#[test]
fn facet_follows_pin() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c"))
        .unwrap()
        .pin(Id::from_raw(0))
        .with(Hardness(5));
    b.create(n("t:a")).unwrap();
    let r = b.build().unwrap();
    let c = r.id("t:c").unwrap();
    let a = r.id("t:a").unwrap();
    assert_eq!(r.column::<Hardness>().unwrap().get(c).unwrap().0, 5);
    assert_eq!(r.column::<Hardness>().unwrap().get(a), None);
}

#[test]
fn or_init_runs_once() {
    let mut b = RegistryBuilder::<Block>::new();
    b.entry(n("t:a")).or_init(|e| e.with(Hardness(1)));
    b.entry(n("t:a")).or_init(|e| e.with(Hardness(2)));
    let r = b.build().unwrap();
    let a = r.id("t:a").unwrap();
    assert_eq!(r.column::<Hardness>().unwrap().get(a).unwrap().0, 1);
}

#[test]
fn insert_column_unsorted_input() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    b.create(n("t:c")).unwrap();
    let mut init = b.build_init().unwrap();
    let (a, b_, c) = (
        init.id("t:a").unwrap(),
        init.id("t:b").unwrap(),
        init.id("t:c").unwrap(),
    );
    init.insert_column([(c, Hardness(3)), (a, Hardness(3)), (b_, Hardness(3))])
        .unwrap();
    assert_eq!(
        init.build()
            .column::<Hardness>()
            .unwrap()
            .get(b_)
            .unwrap()
            .0,
        3
    );
}

struct Twin(Id<Block>);
impl Facet for Twin {}

#[test]
fn twin() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    let base: Vec<(u32, Name)> = b.iter().map(|(i, nm)| (i, nm.clone())).collect();
    let mut twin_of = alloc::vec![];
    for (i, nm) in base {
        let t = b
            .create(alloc::format!("{}_twin", nm.as_str()).parse().unwrap())
            .unwrap();
        twin_of.push((i, t.index()));
    }
    let mut init = b.build_init().unwrap();
    let pairs: Vec<(Id<Block>, Twin)> = twin_of
        .iter()
        .map(|&(base, twin)| (init.perm().id(base), Twin(init.perm().id(twin))))
        .collect();
    init.insert_column(pairs).unwrap();
    let r = init.build();
    assert_eq!(
        r.column::<Twin>().unwrap()[r.id("t:a").unwrap()].0,
        r.id("t:a_twin").unwrap()
    )
}

#[test]
fn cap() {
    let mut b = RegistryBuilder::<Block>::new().with_cap(2);
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    b.build().unwrap();
}

#[test]
fn cap_too_much() {
    let mut b = RegistryBuilder::<Block>::new().with_cap(1);
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::CapExceeded { len: 2, cap: 1 }]
    )
}

#[test]
fn no_props_states_are_ids() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap();
    b.create(n("t:b")).unwrap();
    b.create(n("t:c")).unwrap();
    let r = b.build().unwrap();
    assert_eq!(r.total_states(), 3);
    for (id, _) in r.iter() {
        assert_eq!(r.default_state(id).raw(), id.raw());
        assert_eq!(r.state_count(id), 1);
        assert_eq!(r.entry_of(r.default_state(id)), id);
    }
}

#[test]
fn air_keeps_state_zero() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:air")).unwrap().pin(Id::from_raw(0));
    b.create(n("t:aa")).unwrap().property::<Facing>();
    let r = b.build().unwrap();
    let air = r.id("t:air").unwrap();
    assert_eq!(air.raw(), 0);
    assert_eq!(r.default_state(air).raw(), 0);
    assert_eq!(r.default_state(r.id("t:aa").unwrap()).raw(), 1);
    assert_eq!(r.total_states(), 5);
}

#[test]
fn mixed_radix_roundtrip() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:furnace"))
        .unwrap()
        .property::<Facing>()
        .property::<Lit>();
    let r = b.build().unwrap();
    let id = r.id("t:furnace").unwrap();
    assert_eq!(r.state_count(id), 8);
    for f in [Facing::North, Facing::East, Facing::South, Facing::West] {
        for lit in [false, true] {
            let s = r
                .set(r.set(r.default_state(id), f).unwrap(), Lit(lit))
                .unwrap();
            assert_eq!(r.property::<Facing>(s), Some(f));
            assert_eq!(r.property::<Lit>(s), Some(Lit(lit)));
            assert_eq!(r.entry_of(s), id);
        }
    }
}

#[test]
fn stride_is_declaration_order() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a"))
        .unwrap()
        .property::<Facing>()
        .property::<Lit>();
    let r = b.build().unwrap();
    let id = r.id("t:a").unwrap();
    let s = r.set(r.default_state(id), Facing::East).unwrap();
    let s = r.set(s, Lit(true)).unwrap();
    assert_eq!(s.raw(), 3);
}

#[test]
fn entry_of_boundaries() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().property::<Facing>();
    b.create(n("t:b")).unwrap().property::<Lit>();
    let r = b.build().unwrap();
    let a = r.id("t:a").unwrap();
    let b_ = r.id("t:b").unwrap();
    assert_eq!(r.entry_of(StateId::from_raw(0)), a);
    assert_eq!(r.entry_of(StateId::from_raw(3)), a);
    assert_eq!(r.entry_of(StateId::from_raw(4)), b_);
    assert_eq!(r.entry_of(StateId::from_raw(5)), b_);
}

#[test]
fn property_gating() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().property::<Facing>();
    let r = b.build().unwrap();
    let s = r.default_state(r.id("t:a").unwrap());
    assert_eq!(r.property::<Lit>(s), None);
    assert!(r.set(s, Lit(true)).is_none());
    assert_eq!(r.property::<Facing>(s), Some(Facing::North));
}

#[test]
fn pinned_propful_entry() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:b")).unwrap().property::<Lit>();
    b.create(n("t:a")).unwrap();
    b.create(n("t:z"))
        .unwrap()
        .pin(Id::from_raw(0))
        .property::<Facing>();
    let r = b.build().unwrap();
    assert_eq!(r.default_state(r.id("t:z").unwrap()).raw(), 0);
    assert_eq!(r.default_state(r.id("t:a").unwrap()).raw(), 4);
    assert_eq!(r.default_state(r.id("t:b").unwrap()).raw(), 5);
    assert_eq!(r.total_states(), 7);
}

#[test]
fn minting_order_does_not_matter() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:c")).unwrap().property::<Lit>();
    b.create(n("t:a")).unwrap().property::<Facing>();
    let r = b.build().unwrap();
    assert_eq!(r.state_count(r.id("t:a").unwrap()), 4);
    assert_eq!(r.default_state(r.id("t:c").unwrap()).raw(), 4);
    let s = r
        .set(r.default_state(r.id("t:c").unwrap()), Lit(true))
        .unwrap();
    assert_eq!(s.raw(), 5);
}

#[test]
fn duplicate_property() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a"))
        .unwrap()
        .property::<Facing>()
        .property::<Facing>();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::DuplicateProperty {
            entry: n("t:a"),
            property: type_name::<Facing>()
        }]
    );
}

#[test]
fn state_count_overflow() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a"))
        .unwrap()
        .property::<Huge>()
        .property::<Lit>();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::TooManyStates { entry: n("t:a") }]
    );
}

#[test]
fn total_states_overflow() {
    let mut b = RegistryBuilder::<Block>::new();
    b.create(n("t:a")).unwrap().property::<Huge>();
    b.create(n("t:b")).unwrap().property::<Huge>();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![BuildErrorKind::TooManyStatesTotal { entry: n("t:b") }]
    );
}

#[test]
fn property_errors_accumulate() {
    let mut b = RegistryBuilder::<Block>::new();
    b.declare::<Model>();
    b.create(n("t:a"))
        .unwrap()
        .property::<Facing>()
        .property::<Facing>();
    assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2);
}
