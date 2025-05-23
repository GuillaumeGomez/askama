use std::ops::Deref;

use askama::Template;

#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate<'a> {
    title: &'a str,
}

#[test]
fn test_use_base_directly() {
    let t = BaseTemplate { title: "Foo" };
    assert_eq!(t.render().unwrap(), "Foo\n\nFoo\nCopyright 2017");
}

#[test]
fn test_simple_extends() {
    #[derive(Template)]
    #[template(path = "child.html")]
    struct ChildTemplate<'a> {
        _parent: &'a BaseTemplate<'a>,
    }

    impl<'a> Deref for ChildTemplate<'a> {
        type Target = BaseTemplate<'a>;

        fn deref(&self) -> &Self::Target {
            self._parent
        }
    }

    let t = ChildTemplate {
        _parent: &BaseTemplate { title: "Bar" },
    };
    assert_eq!(
        t.render().unwrap(),
        "Bar\n(Bar) Content goes here\nFoo\nCopyright 2017"
    );
}

#[test]
fn test_empty_child() {
    #[derive(Template)]
    #[template(source = "{% extends \"base.html\" %}", ext = "html")]
    struct EmptyChild<'a> {
        title: &'a str,
    }

    let t = EmptyChild { title: "baz" };
    assert_eq!(t.render().unwrap(), "baz\n\nFoo\nCopyright 2017");
}

pub mod parent {
    use askama::Template;

    #[derive(Template)]
    #[template(path = "base.html")]
    pub struct BaseTemplate<'a> {
        pub title: &'a str,
    }
}

pub mod child {
    use askama::Template;

    use super::parent::*;

    #[derive(Template)]
    #[template(path = "child.html")]
    pub struct ChildTemplate<'a> {
        pub _parent: &'a BaseTemplate<'a>,
    }

    impl<'a> std::ops::Deref for ChildTemplate<'a> {
        type Target = BaseTemplate<'a>;

        fn deref(&self) -> &Self::Target {
            self._parent
        }
    }
}

#[test]
fn test_different_module() {
    let t = child::ChildTemplate {
        _parent: &parent::BaseTemplate { title: "a" },
    };
    assert_eq!(
        t.render().unwrap(),
        "a\n(a) Content goes here\nFoo\nCopyright 2017"
    );
}

#[test]
fn test_nested_blocks() {
    #[derive(Template)]
    #[template(path = "nested-base.html")]
    struct NestedBaseTemplate {}

    #[derive(Template)]
    #[template(path = "nested-child.html")]
    struct NestedChildTemplate {
        _parent: NestedBaseTemplate,
    }

    impl Deref for NestedChildTemplate {
        type Target = NestedBaseTemplate;

        fn deref(&self) -> &Self::Target {
            &self._parent
        }
    }

    let t = NestedChildTemplate {
        _parent: NestedBaseTemplate {},
    };
    assert_eq!(t.render().unwrap(), "\ndurpy\n");
}

#[test]
fn test_deep() {
    #[derive(Template)]
    #[template(path = "deep-base.html")]
    struct DeepBaseTemplate {
        year: u16,
    }

    #[derive(Template)]
    #[template(path = "deep-mid.html")]
    struct DeepMidTemplate {
        _parent: DeepBaseTemplate,
        title: String,
    }

    #[derive(Template)]
    #[template(path = "deep-kid.html")]
    struct DeepKidTemplate {
        _parent: DeepMidTemplate,
        item: String,
    }

    impl Deref for DeepKidTemplate {
        type Target = DeepMidTemplate;

        fn deref(&self) -> &Self::Target {
            &self._parent
        }
    }

    let t = DeepKidTemplate {
        _parent: DeepMidTemplate {
            _parent: DeepBaseTemplate { year: 2018 },
            title: "Test".into(),
        },
        item: "Foo".into(),
    };

    assert_eq!(
        t.render().unwrap(),
        "
<html>
  <head>
  
  <script></script>

  </head>
  <body>
  
  <div id=\"wrap\">
    <section id=\"content\">
    
  Foo Foo Foo

    </section>
    <section id=\"nav\">
      nav nav nav
    </section>
  </div>

  </body>
</html>"
    );
    assert_eq!(
        t._parent.render().unwrap(),
        "
<html>
  <head>
  
  Test
  
    <style></style>
  

  </head>
  <body>
  
  <div id=\"wrap\">
    <section id=\"content\">
    
      No content found
    
    </section>
    <section id=\"nav\">
      nav nav nav
    </section>
  </div>

  </body>
</html>"
    );
    assert_eq!(
        t._parent._parent.render().unwrap(),
        "
<html>
  <head>
  
    <style></style>
  
  </head>
  <body>
  
    nav nav nav
    Copyright 2018
  
  </body>
</html>"
    );
}

#[test]
fn test_flat_deep() {
    #[derive(Template)]
    #[template(path = "deep-base.html")]
    struct FlatDeepBaseTemplate {
        year: u16,
    }

    #[derive(Template)]
    #[template(path = "deep-mid.html")]
    struct FlatDeepMidTemplate {
        title: String,
    }

    #[derive(Template)]
    #[template(path = "deep-kid.html")]
    struct FlatDeepKidTemplate {
        item: String,
    }

    let t = FlatDeepKidTemplate { item: "Foo".into() };

    assert_eq!(
        t.render().unwrap(),
        "
<html>
  <head>
  
  <script></script>

  </head>
  <body>
  
  <div id=\"wrap\">
    <section id=\"content\">
    
  Foo Foo Foo

    </section>
    <section id=\"nav\">
      nav nav nav
    </section>
  </div>

  </body>
</html>"
    );

    let t = FlatDeepMidTemplate {
        title: "Test".into(),
    };
    assert_eq!(
        t.render().unwrap(),
        "
<html>
  <head>
  
  Test
  
    <style></style>
  

  </head>
  <body>
  
  <div id=\"wrap\">
    <section id=\"content\">
    
      No content found
    
    </section>
    <section id=\"nav\">
      nav nav nav
    </section>
  </div>

  </body>
</html>"
    );

    let t = FlatDeepBaseTemplate { year: 2018 };
    assert_eq!(
        t.render().unwrap(),
        "
<html>
  <head>
  
    <style></style>
  
  </head>
  <body>
  
    nav nav nav
    Copyright 2018
  
  </body>
</html>"
    );
}

#[test]
fn test_let_block() {
    #[derive(Template)]
    #[template(path = "let-base.html")]
    #[allow(dead_code)]
    struct LetBase {}

    #[derive(Template)]
    #[template(path = "let-child.html")]
    struct LetChild {}

    let t = LetChild {};
    assert_eq!(t.render().unwrap(), "1");
}

#[test]
fn test_named_end() {
    #[derive(Template)]
    #[template(path = "named-end.html")]
    struct NamedBlocks<'a> {
        title: &'a str,
    }

    let n = NamedBlocks { title: "title" };
    assert_eq!(n.render().unwrap(), "title\n\ntadam\nCopyright 2017");
}
