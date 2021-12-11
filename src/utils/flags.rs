use serde::{
    de::{self, Unexpected, Visitor},
    Deserializer, Serializer,
};
use std::marker::PhantomData;

pub trait Flags:
    std::ops::BitAnd<Output = Self> + std::cmp::Eq + Copy + Default + Extend<Self> + 'static
{
}

pub trait AllFlags: Flags {
    fn all_flags() -> &'static [(Self, &'static str)];
}

pub fn flags_iter<F>(flags: F) -> impl Iterator<Item = (F, &'static str)>
where
    F: Flags + AllFlags,
{
    F::all_flags()
        .iter()
        .copied()
        .filter(move |f| flags & f.0 == f.0)
}

pub fn get_flag_by_name<F>(name: &str) -> Option<F>
where
    F: Flags + AllFlags,
{
    F::all_flags().iter().find(|f| f.1 == name).map(|f| f.0)
}

pub fn serialize_flags<F, S>(flags: &F, serializer: S) -> Result<S::Ok, S::Error>
where
    F: Flags + AllFlags,
    S: Serializer,
{
    //flags_iter(flags).map(|f| f.1).collect::<Vec<_>>()
    serializer.collect_seq(flags_iter(*flags).map(|f| f.1))
}

struct FlagsVisitor<F>(PhantomData<F>);

impl<'de, F> Visitor<'de> for FlagsVisitor<F>
where
    F: Flags + AllFlags,
{
    type Value = F;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("flags as a sequence of strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut flags = F::default();
        while let Some(value) = seq.next_element::<String>()? {
            let flag = get_flag_by_name(&value).ok_or_else(|| {
                let allowed_names = F::all_flags().iter().map(|f| f.1).collect::<Vec<_>>();
                let expected: String = format!("one of {:?}", allowed_names);
                de::Error::invalid_value(Unexpected::Str(&value), &expected.as_str())
            })?;
            flags.extend(std::iter::once(flag));
        }
        Ok(flags)
    }
}

pub fn deserialize_flags<'de, F, D>(deserializer: D) -> Result<F, D::Error>
where
    F: Flags + AllFlags,
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(FlagsVisitor::<F>(PhantomData))
}
