use core::any::type_name;

use crate::{
    BuildError, Facet, Name, Registries, RegistriesError, Storage,
    registries::RegistriesBuildErrorKind, registry::BuildErrorKind,
};

struct Block;
struct Item;
struct Fluid;

struct Hardness(u32);
impl Facet for Hardness {}

struct Model();
impl Facet for Model {
    const STORAGE: Storage<Self> = Storage::Required;
}

fn n(s: &str) -> Name {
    s.parse().unwrap()
}

#[test]
fn two_kinds_roundtrip() {
    let mut b = Registries::builder();
    b.add_kind::<Block>(n("t:block"), |k| {
        k.create(n("t:stone")).unwrap().with(Hardness(5));
    })
    .unwrap()
    .add_kind::<Item>(n("t:item"), |k| {
        k.create(n("t:stick")).unwrap();
    })
    .unwrap();
    let r = b.build().unwrap();
    let blocks = r.get::<Block>().unwrap();
    let stone = blocks.id("t:stone").unwrap();
    assert_eq!(blocks.column::<Hardness>().unwrap()[stone].0, 5);
    assert_eq!(r.get::<Item>().unwrap().len(), 1);
    assert_eq!(r.kind_stable_id::<Block>().unwrap().as_str(), "t:block");
    assert!(r.get::<Fluid>().is_none());
    assert_eq!(r.kind_stable_id::<Fluid>(), None);
}

#[test]
fn duplicate_kind() {
    let mut b = Registries::builder();
    b.add_kind::<Block>(n("t:block"), |_| {}).unwrap();
    assert_eq!(
        b.add_kind::<Block>(n("t:other"), |_| {}).err().unwrap(),
        RegistriesError::DuplicateKind(type_name::<Block>())
    )
}

#[test]
fn duplicate_kind_name() {
    let mut b = Registries::builder();
    b.add_kind::<Block>(n("t:block"), |_| {}).unwrap();
    assert_eq!(
        b.add_kind::<Item>(n("t:block"), |_| {}).err().unwrap(),
        RegistriesError::DuplicateKindStableId(n("t:block"))
    )
}

#[test]
fn build_error_names_the_kind() {
    let mut b = Registries::builder();
    b.add_kind::<Block>(n("t:block"), |k| {
        k.declare::<Model>();
        k.create(n("t:stone")).unwrap();
    })
    .unwrap()
    .add_kind::<Item>(n("t:item"), |k| {
        k.create(n("t:stick")).unwrap();
    })
    .unwrap();
    assert_eq!(
        b.build().map(|_| ()).unwrap_err().errors,
        alloc::vec![RegistriesBuildErrorKind {
            stable_id: n("t:block"),
            build: BuildError {
                errors: alloc::vec![BuildErrorKind::MissingComponent {
                    entry: n("t:stone"),
                    component: type_name::<Model>()
                }]
            }
        }]
    );
}

#[test]
fn two_failing_kinds_both_reported() {
    let mut b = Registries::builder();
    b.add_kind::<Block>(n("t:block"), |k| {
        k.declare::<Model>();
        k.create(n("t:stone")).unwrap();
    })
    .unwrap()
    .add_kind::<Item>(n("t:item"), |k| {
        k.declare::<Model>();
        k.create(n("t:stick")).unwrap();
    })
    .unwrap();
    assert_eq!(b.build().map(|_| ()).unwrap_err().errors.len(), 2)
}
