use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};

use rustc_hash::FxHasher;

use crate::common::Generator;

pub fn first_max_generator<I>(iterator: I, dimension_cutoff: Option<usize>) -> Option<Generator>
where
    I: IntoIterator<Item = Generator>,
{
    let mut max: Option<Generator> = None;

    for generator in iterator {
        if Some(generator.dimension) == dimension_cutoff {
            return Some(generator);
        }

        max = match max {
            Some(prev) if prev.dimension >= generator.dimension => Some(prev),
            _ => Some(generator),
        };
    }

    max
}

pub type Hasher = BuildHasherDefault<FxHasher>;
pub type FastHashMap<K, V> = HashMap<K, V, Hasher>;
pub type FastHashSet<K> = HashSet<K, Hasher>;
