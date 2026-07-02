use core::fmt;

const MULTI_SEGMENT_PREFIX: &str = "segments:";
const ESCAPED_SINGLE_SEGMENT_PREFIX: &str = "segment:";

/// Stable identity for a desired external resource.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ResourceKey {
    inner: Box<ResourceKeyInner>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct ResourceKeyInner {
    segments: Box<[Box<str>]>,
    encoded: Box<str>,
}

impl ResourceKey {
    /// Creates a single-segment resource key from deterministic host-chosen identity.
    pub fn new(key: impl Into<Box<str>>) -> Self {
        Self::from_boxed_segments(vec![key.into()].into_boxed_slice())
    }

    /// Creates a resource key from ordered identity segments.
    ///
    /// Prefer this for product identifiers with multiple parts. Hosts can recover
    /// the exact segments from close commands without parsing a flattened string.
    ///
    /// # Panics
    ///
    /// Panics when called with no segments. Use [`Self::try_from_segments`] when
    /// the segment list may be empty.
    pub fn from_segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        Self::try_from_segments(segments).expect("resource keys require at least one segment")
    }

    /// Creates a resource key from ordered identity segments, returning `None`
    /// when the segment list is empty.
    pub fn try_from_segments<I, S>(segments: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let segments = segments
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        (!segments.is_empty()).then(|| Self::from_boxed_segments(segments))
    }

    /// Creates an explicit broad-resource key.
    ///
    /// Core treats this as an opaque identity; tests and applications decide
    /// whether the key represents a forbidden fallback or wildcard resource.
    pub fn wildcard(key: impl AsRef<str>) -> Self {
        Self::from_segments(["wildcard", key.as_ref()])
    }

    /// Returns this key's deterministic encoded representation.
    ///
    /// Use [`Self::segments`] or [`Self::segment`] when application code needs
    /// product identity back from a resource command. Single-segment keys return
    /// the segment directly unless it is in core's reserved diagnostic namespace.
    pub fn as_str(&self) -> &str {
        &self.inner.encoded
    }

    /// Returns this key's ordered identity segments.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.inner.segments.iter().map(|segment| segment.as_ref())
    }

    /// Returns one identity segment by index.
    pub fn segment(&self, index: usize) -> Option<&str> {
        self.inner
            .segments
            .get(index)
            .map(|segment| segment.as_ref())
    }

    /// Returns the number of identity segments.
    pub fn segment_count(&self) -> usize {
        self.inner.segments.len()
    }

    fn from_boxed_segments(segments: Box<[Box<str>]>) -> Self {
        let encoded = encode_segments(&segments);
        Self {
            inner: Box::new(ResourceKeyInner { segments, encoded }),
        }
    }
}

impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments = self.segments().collect::<Vec<_>>();
        f.debug_tuple("ResourceKey").field(&segments).finish()
    }
}

fn encode_segments(segments: &[Box<str>]) -> Box<str> {
    if let [segment] = segments {
        if segment.starts_with(MULTI_SEGMENT_PREFIX)
            || segment.starts_with(ESCAPED_SINGLE_SEGMENT_PREFIX)
        {
            return encode_single_segment(segment).into_boxed_str();
        }
        return segment.clone();
    }

    let mut encoded = String::from(MULTI_SEGMENT_PREFIX);
    encoded.push_str(&segments.len().to_string());
    encoded.push(':');
    for segment in segments {
        encoded.push_str(&segment.len().to_string());
        encoded.push(':');
        encoded.push_str(segment);
    }
    encoded.into_boxed_str()
}

fn encode_single_segment(segment: &str) -> String {
    let mut encoded = String::from(ESCAPED_SINGLE_SEGMENT_PREFIX);
    encoded.push_str(&segment.len().to_string());
    encoded.push(':');
    encoded.push_str(segment);
    encoded
}

#[cfg(feature = "serde")]
impl serde::Serialize for ResourceKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let [segment] = self.inner.segments.as_ref() {
            serializer.serialize_str(segment)
        } else {
            serializer.collect_seq(self.segments())
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ResourceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum EncodedResourceKey {
            Single(String),
            Segments(Vec<String>),
        }

        match EncodedResourceKey::deserialize(deserializer)? {
            EncodedResourceKey::Single(segment) => Ok(Self::new(segment)),
            EncodedResourceKey::Segments(segments) => {
                Self::try_from_segments(segments).ok_or_else(|| {
                    serde::de::Error::custom("resource key must contain at least one segment")
                })
            }
        }
    }
}
