use std::collections::hash_map::Iter;

use hrana_client_proto::Value;
use serde::{
    de::{value::SeqDeserializer, IntoDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::Row;

fn from_row<'de, T: Deserialize<'de>>(row: &'de Row) -> anyhow::Result<T> {
    let de = De { row };
    T::deserialize(de).map_err(Into::into)
}

struct De<'de> {
    row: &'de Row,
}

impl<'de> Deserializer<'de> for De<'de> {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // visitor.visit_map(RowV { row: &self.row })
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("{}, {:?}", name, fields);

        struct MapA<'a> {
            iter: Iter<'a, String, Value>,
            value: Option<&'a Value>,
        }

        impl<'de> MapAccess<'de> for MapA<'de> {
            type Error = serde::de::value::Error;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: serde::de::DeserializeSeed<'de>,
            {
                if let Some((k, v)) = self.iter.next() {
                    self.value = Some(v);
                    seed.deserialize(k.to_string().into_deserializer())
                        .map(Some)
                } else {
                    Ok(None)
                }
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::DeserializeSeed<'de>,
            {
                let value = self
                    .value
                    .take()
                    .expect("next_value called before next_key");

                seed.deserialize(V(value))
            }
        }

        visitor.visit_map(MapA {
            iter: self.row.value_map.iter(),
            value: None,
        })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

struct V<'a>(&'a Value);

impl<'de> Deserializer<'de> for V<'de> {
    type Error = serde::de::value::Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.0 {
            Value::Text { value } => visitor.visit_string(value.to_string()),
            Value::Null => visitor.visit_unit(),
            Value::Integer { value } => visitor.visit_i64(*value),
            Value::Float { value } => visitor.visit_f64(*value),
            Value::Blob { value } => {
                let seq = SeqDeserializer::new(value.iter().cloned());
                visitor.visit_seq(seq)
            }
            _ => todo!(),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum struct identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(serde::Deserialize)]
    struct Foo {
        bar: String,
        baz: i64,
        baf: f64,
        bab: Vec<u8>,
        ban: (),
    }

    #[test]
    fn smoke() {
        let mut row = Row {
            values: Vec::new(),
            value_map: HashMap::new(),
        };
        row.value_map.insert(
            "bar".to_string(),
            Value::Text {
                value: "foo".into(),
            },
        );
        row.value_map
            .insert("baz".to_string(), Value::Integer { value: 42 });
        row.value_map
            .insert("baf".to_string(), Value::Float { value: 42.0 });
        row.value_map.insert(
            "bab".to_string(),
            Value::Blob {
                value: vec![6u8; 128],
            },
        );
        row.value_map.insert("ban".to_string(), Value::Null);

        from_row::<Foo>(&row).unwrap();
    }
}
