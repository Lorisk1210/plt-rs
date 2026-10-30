use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::rc::Rc;

use super::*;
use crate::{Affine2D, Anchor, Bbox, Path, PathCode, Point, Rgba};

fn round_trip<T: Archive + PartialEq + Debug>(value: &T) -> T {
    let (bytes, externals) = to_bytes(value, &NoProgram).expect("archived");
    let back: T = from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    assert_eq!(&back, value);
    back
}

#[derive(Debug, PartialEq, Default)]
struct Named {
    number: u32,
    text: String,
    cache: Vec<f64>,
}
crate::archive_struct!(Named { number, text } skip { cache });

#[derive(Debug, PartialEq)]
struct Pair(i64, Option<f64>);
crate::archive_struct!(Pair(first, second));

#[derive(Debug, PartialEq)]
struct Unit;
crate::archive_struct!(Unit);

#[derive(Debug, PartialEq)]
struct Wrapper<T>(Vec<T>);
crate::archive_struct!(Wrapper<T>(items));

#[derive(Debug, PartialEq)]
enum Shape {
    Empty,
    Circle(f64),
    Box { corner: Point, size: (f64, f64) },
}
crate::archive_enum!(Shape {
    Empty,
    Circle(radius),
    Box { corner, size },
});

#[test]
fn primitives_round_trip() {
    round_trip(&(true, false, 0u8, 255u8, u64::MAX, usize::MAX));
    round_trip(&(i64::MIN, i64::MAX, -1i32, 0i8, isize::MIN));
    round_trip(&(1.5f64, -0.0f64, f64::INFINITY, f32::MIN_POSITIVE, 'é'));
    round_trip(&(String::from("ünï"), String::new(), ()));
    let (bytes, externals) = to_bytes(&f64::NAN, &NoProgram).expect("archived");
    let nan: f64 = from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    assert_eq!(nan.to_bits(), f64::NAN.to_bits());
}

#[test]
fn containers_round_trip() {
    round_trip(&vec![Some(1u32), None, Some(3)]);
    round_trip(&BTreeMap::from([
        (1, "a".to_string()),
        (2, "b".to_string()),
    ]));
    round_trip(&HashMap::from([("x".to_string(), vec![1.0, 2.0])]));
    round_trip(&[[1.0, 2.0], [3.0, 4.0]]);
    let _ = round_trip(&Box::new(Ok::<u8, String>(4)));
    round_trip(&RefCell::new(Err::<u8, String>("no".into())));
    round_trip(&(0..5usize));
}

#[test]
fn macros_round_trip() {
    let back = round_trip(&Named {
        number: 7,
        text: "seven".into(),
        cache: Vec::new(),
    });
    assert_eq!(back.cache, Vec::<f64>::new());
    round_trip(&Pair(-3, Some(0.5)));
    round_trip(&Unit);
    round_trip(&Wrapper(vec![Pair(1, None), Pair(2, Some(2.0))]));
    round_trip(&vec![
        Shape::Empty,
        Shape::Circle(2.0),
        Shape::Box {
            corner: Point::new(1.0, 2.0),
            size: (3.0, 4.0),
        },
    ]);
}

#[test]
fn skipped_fields_read_back_as_their_default() {
    let named = Named {
        number: 1,
        text: String::new(),
        cache: vec![1.0],
    };
    let (bytes, externals) = to_bytes(&named, &NoProgram).expect("archived");
    let back: Named = from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    assert_eq!(back.cache, Vec::<f64>::new());
}

#[test]
fn core_types_round_trip() {
    round_trip(&Bbox::from_extents(0.0, 1.0, 2.0, 3.0));
    round_trip(&Affine2D::identity().rotate_deg(30.0).translate(1.0, 2.0));
    round_trip(&Rgba::new(0.1, 0.2, 0.3, 0.4));
    round_trip(&vec![Anchor::NE, Anchor::Fraction(0.25, 0.75)]);
    round_trip(&Path::polygon([Point::new(0.0, 0.0), Point::new(1.0, 0.0)]));
    round_trip(&Path::polyline(vec![Point::new(0.0, 1.0)]).with_interpolation_steps(4));
}

#[test]
fn malformed_paths_are_refused() {
    let codes = Some(vec![PathCode::LineTo]);
    let (bytes, externals) =
        to_bytes(&(vec![Point::new(0.0, 0.0)], codes, 1usize), &NoProgram).expect("archived");
    let result: Result<Path, _> = from_bytes(&bytes, &externals, &NoProgram);
    assert!(matches!(result, Err(ArchiveError::Invalid(_))));
}

/// A list shared between values.
type Shared = Rc<RefCell<Vec<i32>>>;

#[test]
fn shared_values_stay_shared() {
    let shared = Rc::new(RefCell::new(vec![1, 2]));
    let value = (
        Rc::clone(&shared),
        Rc::clone(&shared),
        Rc::new(RefCell::new(vec![1, 2])),
    );
    let (bytes, externals) = to_bytes(&value, &NoProgram).expect("archived");
    let (a, b, c): (Shared, Shared, Shared) =
        from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    assert!(Rc::ptr_eq(&a, &b));
    assert!(!Rc::ptr_eq(&a, &c));
    a.borrow_mut().push(3);
    assert_eq!(*b.borrow(), vec![1, 2, 3]);
}

trait Speaker: Any + Debug {
    fn word(&self) -> String;
}

#[derive(Debug, PartialEq)]
struct Dog(u8);
crate::archive_struct!(Dog(times));
impl Speaker for Dog {
    fn word(&self) -> String {
        "woof".repeat(usize::from(self.0))
    }
}

#[derive(Debug)]
struct Cat;
crate::archive_struct!(Cat);
impl Speaker for Cat {
    fn word(&self) -> String {
        "meow".into()
    }
}

/// A speaker pltrs knows nothing about, which the program archives.
#[derive(Debug)]
struct Parrot(String);
impl Speaker for Parrot {
    fn word(&self) -> String {
        self.0.clone()
    }
}

crate::archive_kinds!(dyn Speaker {
    "dog" => Dog,
    "cat" => Cat,
});

/// The program of the tests: it archives parrots (their words as
/// externals) and rebuilds functions doubling numbers.
struct Program;

impl ProgramKinds for Program {
    fn encode_kind(
        &self,
        of: KindOf,
        value: &dyn Any,
        encoder: &mut Encoder<'_>,
    ) -> Result<(), ArchiveError> {
        assert_eq!(of, KindOf::of::<dyn Speaker>("Speaker"));
        let parrot = value
            .downcast_ref::<Parrot>()
            .ok_or_else(|| ArchiveError::unsupported("not a parrot"))?;
        encoder.write_external(&External::new(parrot.0.clone()));
        Ok(())
    }

    fn decode_kind(
        &self,
        _of: KindOf,
        decoder: &mut Decoder<'_>,
    ) -> Result<Box<dyn Any>, ArchiveError> {
        let external = decoder.read_external()?;
        let word = external.downcast_ref::<String>().expect("a word").clone();
        let speaker: Box<dyn Speaker> = Box::new(Parrot(word));
        Ok(Box::new(speaker))
    }

    fn function(&self, function: TypeId, origin: &External) -> Result<Box<dyn Any>, ArchiveError> {
        assert_eq!(function, TypeId::of::<Rc<dyn Fn(f64) -> f64>>());
        let factor = *origin.downcast_ref::<f64>().expect("a factor");
        let function: Rc<dyn Fn(f64) -> f64> = Rc::new(move |x| x * factor);
        Ok(Box::new(function))
    }
}

#[test]
fn trait_objects_archive_their_kind() {
    let speakers: Vec<Box<dyn Speaker>> = vec![Box::new(Dog(2)), Box::new(Cat)];
    let (bytes, externals) = to_bytes(&speakers, &NoProgram).expect("archived");
    let back: Vec<Box<dyn Speaker>> =
        from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    let words: Vec<String> = back.iter().map(|speaker| speaker.word()).collect();
    assert_eq!(words, ["woofwoof", "meow"]);

    let shared: Rc<dyn Speaker> = Rc::new(Dog(1));
    let (bytes, externals) = to_bytes(&(Rc::clone(&shared), shared), &NoProgram).expect("archived");
    let (a, b): (Rc<dyn Speaker>, Rc<dyn Speaker>) =
        from_bytes(&bytes, &externals, &NoProgram).expect("read back");
    assert!(Rc::ptr_eq(&a, &b));
}

#[test]
fn program_kinds_and_functions_go_through_the_program() {
    let parrot: Box<dyn Speaker> = Box::new(Parrot("hello".into()));
    assert!(matches!(
        to_bytes(&parrot, &NoProgram),
        Err(ArchiveError::Unsupported(_))
    ));
    let (bytes, externals) = to_bytes(&parrot, &Program).expect("archived");
    assert_eq!(externals.len(), 1);
    let back: Box<dyn Speaker> = from_bytes(&bytes, &externals, &Program).expect("read back");
    assert_eq!(back.word(), "hello");
    assert!(from_bytes::<Box<dyn Speaker>>(&bytes, &externals, &NoProgram).is_err());

    let double: Rc<dyn Fn(f64) -> f64> = Rc::new(|x| x * 2.0);
    let function = ProgramFn::with_origin(double, External::new(2.0f64));
    let (bytes, externals) = to_bytes(&function, &Program).expect("archived");
    let back: ProgramFn<dyn Fn(f64) -> f64> =
        from_bytes(&bytes, &externals, &Program).expect("read back");
    assert_eq!((back.get())(1.5), 3.0);

    let anonymous = ProgramFn::new(Rc::new(|x: f64| x) as Rc<dyn Fn(f64) -> f64>);
    assert!(matches!(
        to_bytes(&anonymous, &Program),
        Err(ArchiveError::Unsupported(_))
    ));
}

#[test]
fn externals_are_listed_once() {
    let word = External::new("a".to_string());
    let value = vec![
        ProgramFn::with_origin(Rc::new(|x: f64| x) as Rc<dyn Fn(f64) -> f64>, word.clone()),
        ProgramFn::with_origin(Rc::new(|x: f64| x) as Rc<dyn Fn(f64) -> f64>, word),
    ];
    let (_, externals) = to_bytes(&value, &NoProgram).expect("archived");
    assert_eq!(externals.len(), 1);
}

#[test]
fn malformed_archives_are_refused_without_panicking() {
    let value = (vec![1.0f64; 4], "text".to_string(), Shape::Circle(1.0));
    let (bytes, externals) = to_bytes(&value, &NoProgram).expect("archived");
    for len in 0..bytes.len() {
        let result: Result<(Vec<f64>, String, Shape), _> =
            from_bytes(&bytes[..len], &externals, &NoProgram);
        assert!(result.is_err(), "truncated to {len} bytes");
    }
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(from_bytes::<(Vec<f64>, String, Shape)>(&longer, &externals, &NoProgram).is_err());
    assert_eq!(
        from_bytes::<u8>(b"NOTANARCHIVE", &[], &NoProgram),
        Err(ArchiveError::NotAnArchive)
    );
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..2000 {
        let mut corrupted = bytes.clone();
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let index = (seed >> 33) as usize % corrupted.len();
        corrupted[index] ^= (seed >> 8) as u8 | 1;
        let _ = from_bytes::<(Vec<f64>, String, Shape)>(&corrupted, &externals, &NoProgram);
    }
}

#[derive(Debug)]
struct Nest(Option<Box<Nest>>);
crate::archive_struct!(Nest(inner));

#[test]
fn absurd_lengths_and_nesting_are_refused() {
    let mut encoder = Encoder::new(&NoProgram);
    encoder.write_raw(MAGIC);
    encoder.write_varint(FORMAT);
    encoder.write_str(env!("CARGO_PKG_VERSION"));
    encoder.write_varint(u64::MAX >> 1);
    let (bytes, _) = encoder.finish();
    assert_eq!(
        from_bytes::<Vec<u8>>(&bytes, &[], &NoProgram),
        Err(ArchiveError::UnexpectedEnd)
    );

    let mut nest = Nest(None);
    for _ in 0..=MAX_DEPTH {
        nest = Nest(Some(Box::new(nest)));
    }
    let (bytes, externals) = to_bytes(&nest, &NoProgram).expect("archived");
    assert_eq!(
        from_bytes::<Nest>(&bytes, &externals, &NoProgram).err(),
        Some(ArchiveError::TooDeep)
    );
}

#[test]
fn archives_of_another_version_are_refused() {
    let mut encoder = Encoder::new(&NoProgram);
    encoder.write_raw(MAGIC);
    encoder.write_varint(FORMAT);
    encoder.write_str("0.0.0-other");
    let (bytes, _) = encoder.finish();
    assert!(matches!(
        from_bytes::<()>(&bytes, &[], &NoProgram),
        Err(ArchiveError::Version { .. })
    ));
}
