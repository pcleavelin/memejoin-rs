use std::collections::HashMap;

pub trait Build {
    fn build(self) -> String;
}

#[derive(PartialEq)]
pub enum Tag {
    Empty,

    Html,
    Head,
    Link,
    Script,
    Title,
    Body,
    Main,
    Break,

    Details,
    Summary,

    Dialog,
    Article,
    Header,

    Div,

    Table,
    TableHead,
    TableHeader,
    TableBody,
    TableRow,
    TableData,

    Progress,

    Form,
    Label,
    FieldSet,
    Input,
    Select,
    Option,

    Nav,

    OrderedList,
    UnorderedList,
    ListItem,

    Anchor,
    Button,

    Header1,
    Header2,
    Header3,
    Header4,
    Header5,
    Header6,
    Strong,
    Paragraph,
    JustText,
}

impl Tag {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "",
            Self::JustText => "",

            Self::Html => "html",
            Self::Head => "head",
            Self::Link => "link",
            Self::Script => "script",
            Self::Title => "title",
            Self::Body => "body",
            Self::Main => "main",
            Self::Break => "break",

            Self::Progress => "progress",

            Self::Details => "details",
            Self::Summary => "summary",

            Self::Dialog => "dialog",
            Self::Article => "article",
            Self::Header => "header",

            Self::Div => "div",

            Self::Table => "table",
            Self::TableHead => "thead",
            Self::TableHeader => "th",
            Self::TableBody => "tbody",
            Self::TableRow => "tr",
            Self::TableData => "td",

            Self::Form => "form",
            Self::Label => "label",
            Self::FieldSet => "fieldset",
            Self::Input => "input",
            Self::Select => "select",
            Self::Option => "option",

            Self::Nav => "nav",

            Self::OrderedList => "ol",
            Self::UnorderedList => "ul",
            Self::ListItem => "li",

            Self::Anchor => "a",
            Self::Button => "button",

            Self::Header1 => "h1",
            Self::Header2 => "h2",
            Self::Header3 => "h3",
            Self::Header4 => "h4",
            Self::Header5 => "h5",
            Self::Header6 => "h6",
            Self::Strong => "strong",
            Self::Paragraph => "paragraph",
        }
    }

    fn start(&self) -> String {
        if *self != Self::JustText && *self != Self::Empty {
            format!("<{}>", self.as_str())
        } else {
            String::new()
        }
    }

    fn end(&self) -> String {
        if *self != Self::JustText
            && *self != Self::Empty
            && *self != Self::Link
            && *self != Self::Input
        {
            format!("</{}>", self.as_str())
        } else {
            String::new()
        }
    }
}

pub enum SwapMethod {
    InnerHtml,
    OuterHtml,
    BeforeEnd,
    Refresh,
}

impl SwapMethod {
    fn as_str(&self) -> &'static str {
        match self {
            SwapMethod::InnerHtml => "innerHTML",
            SwapMethod::OuterHtml => "outerHTML",
            SwapMethod::BeforeEnd => "beforeend",
            SwapMethod::Refresh => "refresh",
        }
    }
}

pub struct HtmxBuilder {
    tag: Tag,
    attributes: HashMap<String, String>,
    children: Vec<HtmxBuilder>,
    text: Option<String>,
}

impl Build for HtmxBuilder {
    #[must_use]
    fn build(self) -> String {
        let mut string = String::new();

        // TODO: do this better
        {
            if self.tag != Tag::JustText && self.tag != Tag::Empty {
                string.push_str(&format!("<{}", self.tag.as_str()));
            }

            for (attr, value) in self.attributes {
                if value.is_empty() {
                    string.push_str(&format!(" {attr} "));
                } else {
                    string.push_str(&format!(" {attr}='{value}' "));
                }
            }
            if self.tag != Tag::JustText && self.tag != Tag::Empty {
                string.push_str(">");
            }
        }

        if let Some(text) = self.text {
            string.push_str(&text);
        }

        for child in self.children {
            string.push_str(&child.build());
        }

        string.push_str(&self.tag.end());

        string
    }
}

impl HtmxBuilder {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            attributes: HashMap::new(),
            children: Vec::new(),
            text: None,
        }
    }

    pub fn push_builder(mut self, builder: HtmxBuilder) -> Self {
        self.children.push(builder);
        self
    }

    pub fn attribute(mut self, attr: &str, val: &str) -> Self {
        self.attributes.insert(attr.to_string(), val.to_string());
        self
    }

    pub fn hx_get(mut self, uri: &str) -> Self {
        self.attribute("hx-get", uri)
    }

    pub fn hx_post(mut self, uri: &str) -> Self {
        self.attribute("hx-post", uri)
    }

    pub fn hx_swap(mut self, swap_method: SwapMethod) -> Self {
        self.attribute("hx-swap", swap_method.as_str())
    }

    pub fn hx_trigger(mut self, trigger: &str) -> Self {
        self.attribute("hx-trigger", trigger)
    }

    pub fn hx_target(mut self, target: &str) -> Self {
        self.attribute("hx-target", target)
    }

    pub fn html<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Html)));
        self
    }

    pub fn head<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Head)));
        self
    }

    pub fn title(mut self, text: &str) -> HtmxBuilder {
        self.children.push(HtmxBuilder::new(Tag::Title).text(text));
        self
    }

    pub fn body<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Body)));
        self
    }

    pub fn script(mut self, src: &str, integrity: Option<&str>) -> Self {
        let mut b = HtmxBuilder::new(Tag::Script).attribute("src", src);

        if let Some(integrity) = integrity {
            b = b
                .attribute("integrity", integrity)
                .attribute("crossorigin", "anonymous");
        }

        self.children.push(b);
        self
    }

    pub fn style_link(mut self, link: &str) -> Self {
        self.children.push(
            HtmxBuilder::new(Tag::Link)
                .attribute("rel", "stylesheet")
                .attribute("href", link),
        );
        self
    }

    pub fn flag(mut self, flag: &str) -> Self {
        self.attributes.insert(flag.to_string(), "".to_string());
        self
    }

    pub fn builder<F>(mut self, tag: Tag, builder_fn: F) -> Self
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(tag)));
        self
    }

    pub fn builder_text(mut self, tag: Tag, text: &str) -> Self {
        self.children.push(HtmxBuilder::new(tag).text(text));
        self
    }

    pub fn nav<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Nav)));
        self
    }

    pub fn form<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Form)));
        self
    }

    pub fn label<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Label)));
        self
    }

    pub fn input<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children.push(builder_fn(HtmxBuilder::new(Tag::Input)));
        self
    }

    pub fn button<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children
            .push(builder_fn(HtmxBuilder::new(Tag::Button)));
        self
    }

    pub fn ul<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children
            .push(builder_fn(HtmxBuilder::new(Tag::UnorderedList)));
        self
    }

    pub fn li<F>(mut self, builder_fn: F) -> HtmxBuilder
    where
        F: FnOnce(HtmxBuilder) -> HtmxBuilder,
    {
        self.children
            .push(builder_fn(HtmxBuilder::new(Tag::ListItem)));
        self
    }

    pub fn link(mut self, text: &str, href: &str) -> HtmxBuilder {
        // TODO: add href attribute
        self.children.push(
            HtmxBuilder::new(Tag::Anchor)
                .text(text)
                .attribute("href", href),
        );
        self
    }

    pub fn text(mut self, text: &str) -> HtmxBuilder {
        self.text = Some(text.to_string());
        self
    }

    pub fn strong(mut self, text: &str) -> HtmxBuilder {
        self.children.push(HtmxBuilder::new(Tag::Strong).text(text));
        self
    }
}
