#[macro_export]
macro_rules! impl_flags_serde {
    ($type:ty) => {
        impl crate::utils::flags::Flags for $type {}

        impl serde::Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                crate::utils::flags::serialize_flags(self, serializer)
            }
        }
        impl<'de> serde::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                crate::utils::flags::deserialize_flags(deserializer)
            }
        }
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut iter = crate::utils::flags::flags_iter(*self);
                if let Some(flag) = iter.next() {
                    f.write_str(flag.1)?;
                    for flag in iter {
                        f.write_str(", ")?;
                        f.write_str(flag.1)?;
                    }
                }
                Ok(())
            }
        }
    };
}
