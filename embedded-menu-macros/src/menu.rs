use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
    Attribute, DataStruct, DeriveInput, Field, LitStr, Path, Token, Type,
};

struct DataItem {
    label: String,
    details: Option<String>,
    field: Ident,
}

impl Parse for DataItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut label = None;
        let mut details = None;
        let mut field = None;

        while !input.is_empty() {
            let option = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;

            if option == "label" {
                label = Some(input.parse::<LitStr>()?.value());
            } else if option == "details" {
                details = Some(input.parse::<LitStr>()?.value());
            } else if option == "field" {
                field = Some(input.parse::<Ident>()?);
            } else {
                return Err(syn::Error::new_spanned(
                    &option,
                    format!("Unknown option \"{option}\""),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            label: label.unwrap(),
            field: field.unwrap(),
            details,
        })
    }
}

struct NavItem {
    label: String,
    details: Option<String>,
    event: Path,
}

impl Parse for NavItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut label = None;
        let mut details = None;
        let mut event = None;

        while !input.is_empty() {
            let option = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;

            if option == "label" {
                label = Some(input.parse::<LitStr>()?.value());
            } else if option == "details" {
                details = Some(input.parse::<LitStr>()?.value());
            } else if option == "event" {
                event = Some(input.parse::<Path>()?);
            } else {
                return Err(syn::Error::new_spanned(
                    &option,
                    format!("Unknown option \"{option}\""),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            label: label.unwrap(),
            event: event.unwrap(),
            details,
        })
    }
}

enum MenuItemOption {
    Navigation(NavItem),
    Data(DataItem),
}

impl Parse for MenuItemOption {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_kind = input.parse::<Ident>()?;

        let args;
        parenthesized!(args in input);

        if item_kind == "data" {
            Ok(Self::Data(args.parse::<DataItem>()?))
        } else if item_kind == "navigation" {
            Ok(Self::Navigation(args.parse::<NavItem>()?))
        } else {
            return Err(syn::Error::new_spanned(
                &item_kind,
                format!("Unknown menu item kind \"{item_kind}\""),
            ));
        }
    }
}

impl TryFrom<&Field> for MenuItemOption {
    type Error = syn::Error;

    fn try_from(field: &Field) -> Result<Self, Self::Error> {
        let Some(ident) = field.ident.clone() else {
            return Err(syn::Error::new(Span::call_site(), "Menu can only be placed on named structs"));
        };

        Ok(Self::Data(DataItem {
            label: ident.to_string(),
            details: None,
            field: ident,
        }))
    }
}

enum MenuItem {
    Nav(NavItem),
    Data { data: DataItem, ty: Path },
}
impl MenuItem {
    fn menu_item_in_ty(&self, events: &Ident) -> TokenStream {
        match self {
            MenuItem::Nav { .. } => quote!(NavigationItem<'static, #events>),
            MenuItem::Data { ty, .. } => quote!(Select<'static, #events, #ty>),
        }
    }

    fn as_data_field(&self) -> Option<&Ident> {
        let Self::Data { data: DataItem { field, .. }, .. } = self else {
            return None;
        };

        Some(field)
    }

    fn menu_item(&self, events: &Ident) -> TokenStream {
        match self {
            MenuItem::Nav(NavItem {
                label,
                event,
                details: None,
            }) => quote! {
                NavigationItem::new(#label, #events::__NavigationEvent(#event))
            },
            MenuItem::Nav(NavItem {
                label,
                event,
                details: Some(details),
            }) => quote! {
                NavigationItem::new(#label, #events::__NavigationEvent(#event)).with_detail_text(#details)
            },
            MenuItem::Data {
                data:
                    DataItem {
                        field,
                        label,
                        details: None,
                    },
                ..
            } => quote! {
                Select::new(#label, self.#field)
                    .with_value_converter(#events::#field),
            },

            MenuItem::Data {
                data:
                    DataItem {
                        field,
                        label,
                        details: Some(details),
                    },
                ..
            } => quote! {
                Select::new(#label, self.#field)
                    .with_value_converter(#events::#field)
                    .with_detail_text(#details),
            },
        }
    }

    fn as_enum_variant(&self) -> Option<TokenStream> {
        if let MenuItem::Data {
            data: DataItem { field, .. },
            ty,
        } = self
        {
            Some(quote!(#field(#ty)))
        } else {
            None
        }
    }
}

struct MenuOptions {
    title: String,
    items: Vec<MenuItemOption>,
    external_nav_event_ty: Option<Path>,
}

impl MenuOptions {
    fn contains(&self, ident: &Ident) -> bool {
        for item in self.items.iter() {
            if let MenuItemOption::Data(DataItem { field, .. }) = item {
                if field == ident {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for MenuOptions {
    fn default() -> Self {
        Self {
            title: String::from("Menu"),
            items: Vec::new(),
            external_nav_event_ty: None,
        }
    }
}

impl Parse for MenuOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Self {
            title: String::new(),
            items: Vec::new(),
            external_nav_event_ty: None,
        };

        while !input.is_empty() {
            let option = input.parse::<Ident>()?;

            if option == "title" {
                if !options.title.is_empty() {
                    return Err(syn::Error::new_spanned(option, "Title is already set."));
                }

                let _ = input.parse::<Token![=]>()?;
                options.title = input.parse::<LitStr>()?.value();
            } else if option == "event" {
                if !options.external_nav_event_ty.is_none() {
                    return Err(syn::Error::new_spanned(
                        option,
                        "Event type is already set.",
                    ));
                }

                let _ = input.parse::<Token![=]>()?;
                options.external_nav_event_ty = Some(input.parse::<Path>()?);
            } else if option == "items" {
                if !options.items.is_empty() {
                    return Err(syn::Error::new_spanned(option, "Items are already set."));
                }

                let _ = input.parse::<Token![=]>()?;

                let items;
                bracketed!(items in input);

                while !items.is_empty() {
                    let item = items.parse::<MenuItemOption>()?;

                    if !items.is_empty() {
                        let _ = items.parse::<Token![,]>()?;
                    }

                    options.items.push(item);
                }
            } else {
                return Err(syn::Error::new_spanned(
                    &option,
                    format!("Unknown option \"{option}\""),
                ));
            }

            if !input.is_empty() {
                let _ = input.parse::<Token![,]>()?;
            }
        }

        Ok(options)
    }
}

struct MenuData {
    ty_name: Ident,
    title: String,
    external_nav_event_ty: Option<Path>,
    items: Vec<MenuItem>,
}

impl TryFrom<MenuInput> for MenuData {
    type Error = syn::Error;

    fn try_from(input: MenuInput) -> Result<Self, Self::Error> {
        let ty_name = input.ident;

        let mut attributes = input
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("menu"));

        let attribute = attributes.next();

        if let Some(second) = attributes.next() {
            return Err(syn::Error::new_spanned(
                second,
                "Only one \"menu\" attribute is allowed",
            ));
        }

        let mut menu_options = if let Some(attribute) = attribute {
            attribute.parse_args()?
        } else {
            MenuOptions::default()
        };

        for item in menu_options.items.iter() {
            if let MenuItemOption::Data(DataItem { field, .. }) = item {
                if !input
                    .data
                    .fields
                    .iter()
                    .any(|f| f.ident.as_ref() == Some(field))
                {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!("Field \"{field}\" is not a member of the struct"),
                    ));
                }
            }
        }

        // Collect undecorated fields at the bottom
        for field in input.data.fields.iter() {
            if !menu_options.contains(field.ident.as_ref().unwrap()) {
                menu_options.items.push(MenuItemOption::try_from(field)?);
            }
        }

        let mut items = Vec::new();

        for item in menu_options.items {
            match item {
                MenuItemOption::Navigation(nav) => items.push(MenuItem::Nav(nav)),
                MenuItemOption::Data(data) => {
                    for field in input.data.fields.iter() {
                        if field.ident.as_ref() == Some(&data.field) {
                            let Type::Path(ty) = &field.ty else {
                                return Err(syn::Error::new_spanned(
                                    field,
                                    "Field must be of type bool or enum",
                                ));
                            };

                            items.push(MenuItem::Data {
                                data,
                                ty: ty.path.clone(),
                            });
                            break;
                        }
                    }
                }
            }
        }

        Ok(Self {
            title: menu_options.title,
            ty_name,
            external_nav_event_ty: menu_options.external_nav_event_ty,
            items,
        })
    }
}

struct MenuInput {
    ident: Ident,
    attrs: Vec<Attribute>,
    data: DataStruct,
}

pub fn expand_menu(input: DeriveInput) -> syn::Result<TokenStream> {
    let syn::Data::Struct(data) = input.data else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Menu can only be placed on non-generic structs",
        ));
    };

    let input = MenuInput {
        ident: input.ident,
        attrs: input.attrs,
        data,
    };

    let menu_data = MenuData::try_from(input)?;

    let wrapper = format_ident!("{}MenuWrapper", menu_data.ty_name);
    let events = format_ident!("{}MenuEvents", menu_data.ty_name);
    let module = format_ident!("{}_module", menu_data.ty_name);

    let event_variants = menu_data
        .items
        .iter()
        .filter_map(|item| item.as_enum_variant());
    let menu_items_in_ty = menu_data
        .items
        .iter()
        .map(|item| item.menu_item_in_ty(&events));
    let event_set_data_fields = menu_data
        .items
        .iter()
        .filter_map(|item| item.as_data_field());
    let menu_items = menu_data.items.iter().map(|item| item.menu_item(&events));

    let title = menu_data.title;
    let ty_name = menu_data.ty_name;
    let external_nav_event_ty = if let Some(ty) = menu_data.external_nav_event_ty {
        quote! {#ty}
    } else {
        quote! {()}
    };

    Ok(quote! {
        #[allow(non_snake_case)]
        mod #module {
            use super::*;

            use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
            use embedded_layout::object_chain::*;
            use embedded_menu::{
                builder::MenuBuilder,
                interaction::{programmed::Programmed, InteractionController},
                items::{MenuLine, NavigationItem, Select},
                selection_indicator::{
                    style::{line::Line, IndicatorStyle},
                    SelectionIndicatorController, StaticPosition,
                },
                Menu, MenuStyle, NoItems,
            };

            #[derive(Clone, Copy)]
            #[allow(non_camel_case_types)]
            enum #events {
                __NavigationEvent(#external_nav_event_ty),
                #(#event_variants),*
            }

            pub struct #wrapper<IT, P, S>
            where
                IT: InteractionController,
                P: SelectionIndicatorController,
                S: IndicatorStyle,
            {
                menu: Menu<
                    IT,
                    embedded_layout::chain! {
                        #(MenuLine<#menu_items_in_ty>),*
                    },
                    #events,
                    BinaryColor,
                    P,
                    S,
                >,
                data: DemoMenu,
            }

            impl<IT, P, S> #wrapper<IT, P, S>
            where
                IT: InteractionController,
                P: SelectionIndicatorController,
                S: IndicatorStyle,
            {
                pub fn data(&self) -> DemoMenu {
                    self.data.clone()
                }

                pub fn interact(&mut self, event: IT::Input) -> Option<#external_nav_event_ty> {
                    match self.menu.interact(event)? {
                        #(#events::#event_set_data_fields(value) => self.data.#event_set_data_fields = value,)*
                        #events::__NavigationEvent(event) => return Some(event),
                    };

                    None
                }

                pub fn update(&mut self, display: &impl Dimensions) {
                    self.menu.update(display)
                }
            }

            impl<IT, P, S> Drawable for #wrapper<IT, P, S>
            where
                IT: InteractionController,
                P: SelectionIndicatorController,
                S: IndicatorStyle,
            {
                type Color = BinaryColor;
                type Output = ();

                fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
                where
                    D: DrawTarget<Color = BinaryColor>,
                {
                    self.menu.draw(display)
                }
            }

            impl #ty_name {
                fn setup_menu<S, IT, P>(
                    self,
                    builder: MenuBuilder<IT, NoItems, #events, BinaryColor, P, S>,
                ) -> #wrapper<IT, P, S>
                where
                    S: IndicatorStyle,
                    IT: InteractionController,
                    P: SelectionIndicatorController,
                {
                    #wrapper {
                        data: self,
                        menu: builder
                            #(.add_item(#menu_items))*
                            .build(),
                    }
                }

                pub fn create_menu(self) -> #wrapper<Programmed, StaticPosition, Line> {
                    self.create_menu_with_style(MenuStyle::default())
                }

                pub fn create_menu_with_style<S, IT, P>(
                    self,
                    style: MenuStyle<BinaryColor, S, IT, P>,
                ) -> #wrapper<IT, P, S>
                where
                    S: IndicatorStyle,
                    IT: InteractionController,
                    P: SelectionIndicatorController,
                {
                    let builder = Menu::with_style(#title, style);
                    self.setup_menu(builder)
                }
            }
        }

        pub use #module::#wrapper;
    })
}