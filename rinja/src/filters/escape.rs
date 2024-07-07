use std::convert::Infallible;
use std::fmt::{self, Display, Formatter, Write};
use std::num::NonZeroU8;
use std::str;

/// Marks a string (or other `Display` type) as safe
///
/// Use this if you want to allow markup in an expression, or if you know
/// that the expression's contents don't need to be escaped.
///
/// Rinja will automatically insert the first (`Escaper`) argument,
/// so this filter only takes a single argument of any type that implements
/// `Display`.
#[inline]
pub fn safe(text: impl fmt::Display, escaper: impl Escaper) -> Result<impl Display, Infallible> {
    let _ = escaper; // it should not be part of the interface that the `escaper` is unused
    Ok(text)
}

/// Escapes strings according to the escape mode.
///
/// Rinja will automatically insert the first (`Escaper`) argument,
/// so this filter only takes a single argument of any type that implements
/// `Display`.
///
/// It is possible to optionally specify an escaper other than the default for
/// the template's extension, like `{{ val|escape("txt") }}`.
#[inline]
pub fn escape(text: impl fmt::Display, escaper: impl Escaper) -> Result<impl Display, Infallible> {
    Ok(EscapeDisplay(text, escaper))
}

pub struct EscapeDisplay<T, E>(T, E);

impl<T: fmt::Display, E: Escaper> fmt::Display for EscapeDisplay<T, E> {
    #[inline]
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        struct EscapeWriter<W, E>(W, E);

        impl<W: Write, E: Escaper> Write for EscapeWriter<W, E> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.1.write_escaped_str(&mut self.0, s)
            }

            #[inline]
            fn write_char(&mut self, c: char) -> fmt::Result {
                self.1.write_escaped_char(&mut self.0, c)
            }
        }

        write!(EscapeWriter(fmt, self.1), "{}", &self.0)
    }
}

/// Alias for [`escape()`]
#[inline]
pub fn e(text: impl fmt::Display, escaper: impl Escaper) -> Result<impl Display, Infallible> {
    escape(text, escaper)
}

/// Escape characters in a safe way for HTML texts and attributes
///
/// * `"` => `&#34;`
/// * `&` => `&#38;`
/// * `'` => `&#39;`
/// * `<` => `&#60;`
/// * `>` => `&#62;`
#[derive(Debug, Clone, Copy, Default)]
pub struct Html;

impl Escaper for Html {
    fn write_escaped_str<W: Write>(&self, mut fmt: W, string: &str) -> fmt::Result {
        let mut escaped_buf = *b"&#__;";
        let mut last = 0;

        for (index, byte) in string.bytes().enumerate() {
            const MIN_CHAR: u8 = b'"';
            const MAX_CHAR: u8 = b'>';

            struct Table {
                _align: [usize; 0],
                lookup: [Option<[NonZeroU8; 2]>; (MAX_CHAR - MIN_CHAR + 1) as usize],
            }

            const TABLE: Table = {
                const fn n(c: u8) -> Option<[NonZeroU8; 2]> {
                    let n0 = match NonZeroU8::new(c / 10 + b'0') {
                        Some(n) => n,
                        None => panic!(),
                    };
                    let n1 = match NonZeroU8::new(c % 10 + b'0') {
                        Some(n) => n,
                        None => panic!(),
                    };
                    Some([n0, n1])
                }

                let mut table = Table {
                    _align: [],
                    lookup: [None; (MAX_CHAR - MIN_CHAR + 1) as usize],
                };

                table.lookup[(b'"' - MIN_CHAR) as usize] = n(b'"');
                table.lookup[(b'&' - MIN_CHAR) as usize] = n(b'&');
                table.lookup[(b'\'' - MIN_CHAR) as usize] = n(b'\'');
                table.lookup[(b'<' - MIN_CHAR) as usize] = n(b'<');
                table.lookup[(b'>' - MIN_CHAR) as usize] = n(b'>');
                table
            };

            let escaped = match byte {
                MIN_CHAR..=MAX_CHAR => TABLE.lookup[(byte - MIN_CHAR) as usize],
                _ => None,
            };
            if let Some(escaped) = escaped {
                escaped_buf[2] = escaped[0].get();
                escaped_buf[3] = escaped[1].get();
                fmt.write_str(&string[last..index])?;
                fmt.write_str(unsafe { std::str::from_utf8_unchecked(escaped_buf.as_slice()) })?;
                last = index + 1;
            }
        }
        fmt.write_str(&string[last..])
    }

    fn write_escaped_char<W: Write>(&self, mut fmt: W, c: char) -> fmt::Result {
        fmt.write_str(match (c.is_ascii(), c as u8) {
            (true, b'"') => "&#34;",
            (true, b'&') => "&#38;",
            (true, b'\'') => "&#39;",
            (true, b'<') => "&#60;",
            (true, b'>') => "&#62;",
            _ => return fmt.write_char(c),
        })
    }
}

/// Don't escape the input but return in verbatim
#[derive(Debug, Clone, Copy, Default)]
pub struct Text;

impl Escaper for Text {
    #[inline]
    fn write_escaped_str<W: Write>(&self, mut fmt: W, string: &str) -> fmt::Result {
        fmt.write_str(string)
    }

    #[inline]
    fn write_escaped_char<W: Write>(&self, mut fmt: W, c: char) -> fmt::Result {
        fmt.write_char(c)
    }
}

pub trait Escaper: Copy {
    fn write_escaped_str<W: Write>(&self, fmt: W, string: &str) -> fmt::Result;

    #[inline]
    fn write_escaped_char<W: Write>(&self, fmt: W, c: char) -> fmt::Result {
        self.write_escaped_str(fmt, c.encode_utf8(&mut [0; 4]))
    }
}

/// Used internally by rinja to select the appropriate escaper
pub trait AutoEscape {
    type Escaped: fmt::Display;
    type Error: Into<crate::Error>;

    fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error>;
}

/// Used internally by rinja to select the appropriate escaper
#[derive(Debug, Clone)]
pub struct AutoEscaper<'a, T: fmt::Display + ?Sized, E: Escaper> {
    text: &'a T,
    escaper: E,
}

impl<'a, T: fmt::Display + ?Sized, E: Escaper> AutoEscaper<'a, T, E> {
    #[inline]
    pub fn new(text: &'a T, escaper: E) -> Self {
        Self { text, escaper }
    }
}

/// Use the provided escaper
impl<'a, T: fmt::Display + ?Sized, E: Escaper> AutoEscape for &&AutoEscaper<'a, T, E> {
    type Escaped = EscapeDisplay<&'a T, E>;
    type Error = Infallible;

    #[inline]
    fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error> {
        Ok(EscapeDisplay(self.text, self.escaper))
    }
}

/// Types that implement this marker trait don't need to be HTML escaped
///
/// Please note that this trait is only meant as speed-up helper. In some odd circumcises rinja
/// might still decide to HTML escape the input, so if this must not happen, then you need to use
/// the [`|safe`](super::safe) filter to prevent the auto escaping.
///
/// If you are unsure if your type generates HTML safe output in all cases, then DON'T mark it.
/// Better safe than sorry!
pub trait HtmlSafeMarker: fmt::Display {}

impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for &T {}

/// Don't escape HTML safe types
impl<'a, T: HtmlSafeMarker + ?Sized> AutoEscape for &AutoEscaper<'a, T, Html> {
    type Escaped = &'a T;
    type Error = Infallible;

    #[inline]
    fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error> {
        Ok(self.text)
    }
}

/// Mark the output of a filter as "maybe safe"
///
/// This struct can be used as a transparent return type of custom filters that want to mark
/// their output as "safe" depending on some circumstances, i.e. that their output maybe does not
/// need to be escaped.
///
/// If the filter is not used as the last element in the filter chain, then any assumption is void.
/// Let the next filter decide if the output is safe or not.
pub struct MaybeSafe<T: fmt::Display> {
    pub text: T,
    pub safe: bool,
}

const _: () = {
    // This is the fallback. The filter is not the last element of the filter chain.
    impl<T: fmt::Display> fmt::Display for MaybeSafe<T> {
        #[inline]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.text)
        }
    }

    impl<'a, T: fmt::Display, E: Escaper> AutoEscape for &AutoEscaper<'a, MaybeSafe<T>, E> {
        type Escaped = Wrapped<'a, T, E>;
        type Error = Infallible;

        #[inline]
        fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error> {
            match self.text.safe {
                true => Ok(Wrapped::Safe(&self.text.text)),
                false => Ok(Wrapped::Unsafe(&self.text.text, self.escaper)),
            }
        }
    }

    pub enum Wrapped<'a, T: fmt::Display + ?Sized, E: Escaper> {
        Unsafe(&'a T, E),
        Safe(&'a T),
    }

    impl<T: fmt::Display + ?Sized, E: Escaper> fmt::Display for Wrapped<'_, T, E> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match *self {
                Self::Unsafe(text, escaper) => EscapeDisplay(text, escaper).fmt(f),
                Self::Safe(text) => write!(f, "{text}"),
            }
        }
    }
};

/// Mark the output of a filter as "safe"
///
/// This struct can be used as a transparent return type of custom filters that want to mark their
/// output as "safe" no matter what, i.e. that their output does not need to be escaped.
///
/// If the filter is not used as the last element in the filter chain, then any assumption is void.
/// Let the next filter decide if the output is safe or not.
pub struct Safe<T: fmt::Display>(pub T);

const _: () = {
    // This is the fallback. The filter is not the last element of the filter chain.
    impl<T: fmt::Display> fmt::Display for Safe<T> {
        #[inline]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<'a, T: fmt::Display, E: Escaper> AutoEscape for &AutoEscaper<'a, Safe<T>, E> {
        type Escaped = &'a T;
        type Error = Infallible;

        #[inline]
        fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error> {
            Ok(&self.text.0)
        }
    }
};

/// There is not need to mark the output of a custom filter as "unsafe"; this is simply the default
pub struct Unsafe<T: fmt::Display>(pub T);

const _: () = {
    // This is the fallback. The filter is not the last element of the filter chain.
    impl<T: fmt::Display> fmt::Display for Unsafe<T> {
        #[inline]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<'a, T: fmt::Display, E: Escaper> AutoEscape for &AutoEscaper<'a, Unsafe<T>, E> {
        type Escaped = EscapeDisplay<&'a T, E>;
        type Error = Infallible;

        #[inline]
        fn rinja_auto_escape(&self) -> Result<Self::Escaped, Self::Error> {
            Ok(EscapeDisplay(&self.text.0, self.escaper))
        }
    }
};

macro_rules! mark_html_safe {
    ($($ty:ty),* $(,)?) => {$(
        impl HtmlSafeMarker for $ty {}
    )*};
}

mark_html_safe! {
    bool,
    f32, f64,
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    std::num::NonZeroI8, std::num::NonZeroI16, std::num::NonZeroI32,
    std::num::NonZeroI64, std::num::NonZeroI128, std::num::NonZeroIsize,
    std::num::NonZeroU8, std::num::NonZeroU16, std::num::NonZeroU32,
    std::num::NonZeroU64, std::num::NonZeroU128, std::num::NonZeroUsize,
}

impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for Box<T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::cell::Ref<'_, T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::cell::RefMut<'_, T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::rc::Rc<T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::sync::Arc<T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::sync::MutexGuard<'_, T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::sync::RwLockReadGuard<'_, T> {}
impl<T: HtmlSafeMarker + ?Sized> HtmlSafeMarker for std::sync::RwLockWriteGuard<'_, T> {}
impl<T: HtmlSafeMarker> HtmlSafeMarker for std::num::Wrapping<T> {}

impl<T> HtmlSafeMarker for std::borrow::Cow<'_, T>
where
    T: HtmlSafeMarker + std::borrow::ToOwned + ?Sized,
    T::Owned: HtmlSafeMarker,
{
}

#[test]
fn test_escape() {
    assert_eq!(escape("", Html).unwrap().to_string(), "");
    assert_eq!(escape("<&>", Html).unwrap().to_string(), "&#60;&#38;&#62;");
    assert_eq!(escape("bla&", Html).unwrap().to_string(), "bla&#38;");
    assert_eq!(escape("<foo", Html).unwrap().to_string(), "&#60;foo");
    assert_eq!(escape("bla&h", Html).unwrap().to_string(), "bla&#38;h");

    assert_eq!(escape("", Text).unwrap().to_string(), "");
    assert_eq!(escape("<&>", Text).unwrap().to_string(), "<&>");
    assert_eq!(escape("bla&", Text).unwrap().to_string(), "bla&");
    assert_eq!(escape("<foo", Text).unwrap().to_string(), "<foo");
    assert_eq!(escape("bla&h", Text).unwrap().to_string(), "bla&h");
}

#[test]
fn test_html_safe_marker() {
    struct Script1;
    struct Script2;

    impl fmt::Display for Script1 {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("<script>")
        }
    }

    impl fmt::Display for Script2 {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("<script>")
        }
    }

    impl HtmlSafeMarker for Script2 {}

    assert_eq!(
        (&&AutoEscaper::new(&Script1, Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "&lt;script&gt;",
    );
    assert_eq!(
        (&&AutoEscaper::new(&Script2, Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );

    assert_eq!(
        (&&AutoEscaper::new(&Script1, Text))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );
    assert_eq!(
        (&&AutoEscaper::new(&Script2, Text))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );

    assert_eq!(
        (&&AutoEscaper::new(&Safe(Script1), Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );
    assert_eq!(
        (&&AutoEscaper::new(&Safe(Script2), Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );

    assert_eq!(
        (&&AutoEscaper::new(&Unsafe(Script1), Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "&lt;script&gt;",
    );
    assert_eq!(
        (&&AutoEscaper::new(&Unsafe(Script2), Html))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "&lt;script&gt;",
    );

    assert_eq!(
        (&&AutoEscaper::new(
            &MaybeSafe {
                safe: true,
                text: Script1
            },
            Html
        ))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );
    assert_eq!(
        (&&AutoEscaper::new(
            &MaybeSafe {
                safe: true,
                text: Script2
            },
            Html,
        ))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "<script>",
    );
    assert_eq!(
        (&&AutoEscaper::new(
            &MaybeSafe {
                safe: false,
                text: Script1
            },
            Html,
        ))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "&lt;script&gt;",
    );
    assert_eq!(
        (&&AutoEscaper::new(
            &MaybeSafe {
                safe: false,
                text: Script2
            },
            Html,
        ))
            .rinja_auto_escape()
            .unwrap()
            .to_string(),
        "&lt;script&gt;",
    );
}
