use axum_liveview::{html, messages::InputEvent, Html};
use std::{fmt, str::FromStr};

#[derive(Debug)]
pub(crate) struct Input<T, V> {
    name: String,
    type_: &'static str,
    value: Option<T>,
    validation: V,
    ever_updated: bool,
    validation_errors: Vec<String>,
}

impl<T> Input<T, AlwaysValid> {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            type_: "text",
            value: None,
            validation: always_valid(),
            ever_updated: false,
            validation_errors: Default::default(),
        }
    }
}

impl<T, V> Input<T, V>
where
    V: Validation<T>,
{
    pub(crate) fn type_(mut self, type_: &'static str) -> Self {
        self.type_ = type_;
        self
    }

    pub(crate) fn changed_topic(&self) -> String {
        format!("input-{}-changed", self.name)
    }

    pub(crate) fn blur_topic(&self) -> String {
        format!("input-{}-blur", self.name)
    }

    pub(crate) fn focus_topic(&self) -> String {
        format!("input-{}-focus", self.name)
    }

    pub(crate) fn validation<V2>(self, validation: V2) -> Input<T, V2>
    where
        V2: Validation<T>,
    {
        Input {
            name: self.name,
            type_: self.type_,
            value: self.value,
            ever_updated: self.ever_updated,
            validation_errors: self.validation_errors,
            validation,
        }
    }

    pub(crate) fn render(&self) -> Html
    where
        T: fmt::Display,
    {
        html! {
            <div>
                <label>
                    <div>{ &self.name }</div>
                    <input
                        type={ self.type_ }
                        name={ &self.name }
                        value={
                            self.value
                                .as_ref()
                                .map(|value| value.to_string())
                                .unwrap_or_default()
                        }
                        live-input={ self.changed_topic() }
                        live-blur={ self.blur_topic() }
                        live-focus={ self.focus_topic() }
                    />
                </label>

                if !self.validation_errors.is_empty() {
                    <div>
                        "Errors:"
                        <ul>
                            for error in &self.validation_errors {
                                <li>{ format!("{} {}", self.name, error) }</li>
                            }
                        </ul>
                    </div>
                }
            </div>
        }
    }

    pub(crate) fn update_value(&mut self, event: InputEvent)
    where
        T: FromStr,
    {
        self.ever_updated = true;

        if let Ok(value) = event.value().parse() {
            self.value = Some(value);
        } else {
            self.value = None;
        }
    }

    pub(crate) fn update_validations(&mut self, _event: InputEvent) {
        self.ever_updated = true;

        self.validation_errors.clear();
        if self.ever_updated {
            self.validation
                .validate(self.value.as_ref(), &mut self.validation_errors);
        }
    }
}

pub(crate) trait Validation<T> {
    fn validate(&self, value: Option<&T>, errors: &mut Vec<String>);

    fn boxed(self) -> BoxedValidation<T>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Box::new(self)
    }

    fn and<K>(self, other: K) -> And<Self, K>
    where
        K: Validation<T>,
        Self: Sized,
    {
        And {
            lhs: self,
            rhs: other,
        }
    }
}

pub(crate) type BoxedValidation<T> = Box<dyn Validation<T> + Send + Sync>;

impl<T> Validation<T> for BoxedValidation<T> {
    fn validate(&self, value: Option<&T>, errors: &mut Vec<String>) {
        Validation::validate(&**self, value, errors)
    }
}

pub(crate) struct And<A, B> {
    lhs: A,
    rhs: B,
}

impl<T, A, B> Validation<T> for And<A, B>
where
    A: Validation<T>,
    B: Validation<T>,
{
    fn validate(&self, value: Option<&T>, errors: &mut Vec<String>) {
        self.lhs.validate(value, errors);
        self.rhs.validate(value, errors);
    }
}

pub(crate) struct AlwaysValid(());

#[allow(dead_code)]
pub(crate) fn always_valid() -> AlwaysValid {
    AlwaysValid(())
}

impl<T> Validation<T> for AlwaysValid {
    fn validate(&self, _value: Option<&T>, _errors: &mut Vec<String>) {}
}

pub(crate) struct NotEmpty(());

pub(crate) fn not_empty() -> NotEmpty {
    NotEmpty(())
}

impl Validation<String> for NotEmpty {
    fn validate(&self, value: Option<&String>, errors: &mut Vec<String>) {
        if let Some(value) = value {
            if value.is_empty() {
                errors.push("cannot be empty".to_owned());
                return;
            }

            if value.chars().all(|c| c.is_whitespace()) {
                errors.push("cannot contain only whitespace".to_owned());
            }
        }
    }
}

pub(crate) struct Present(());

pub(crate) fn present() -> Present {
    Present(())
}

impl<T> Validation<T> for Present {
    fn validate(&self, value: Option<&T>, errors: &mut Vec<String>) {
        if value.is_none() {
            errors.push("cannot be empty".to_owned());
        }
    }
}

pub(crate) struct GreaterThan(u32);

pub(crate) fn greater_than(bound: u32) -> GreaterThan {
    GreaterThan(bound)
}

impl Validation<u32> for GreaterThan {
    fn validate(&self, value: Option<&u32>, errors: &mut Vec<String>) {
        if let Some(value) = value {
            if *value <= self.0 {
                errors.push(format!("must be greater than {}", self.0))
            }
        }
    }
}

pub(crate) struct LessThan(u32);

pub(crate) fn less_than(bound: u32) -> LessThan {
    LessThan(bound)
}

impl Validation<u32> for LessThan {
    fn validate(&self, value: Option<&u32>, errors: &mut Vec<String>) {
        if let Some(value) = value {
            if *value >= self.0 {
                errors.push(format!("must be less than {}", self.0))
            }
        }
    }
}
