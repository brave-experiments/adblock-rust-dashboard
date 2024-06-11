#![recursion_limit="1024"]

use adblock::resources::PermissionMask;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::services::{Task, TimeoutService};

use adblock::lists::{parse_filter, FilterFormat, FilterParseError, ParsedFilter, ParseOptions, RuleTypes};
use adblock::content_blocking::{CbRuleEquivalent, CbRuleCreationFailure};

mod util;

struct Model {
    link: ComponentLink<Self>,

    filter: String,
    parse_result: Result<ParsedFilter, FilterParseError>,
    cb_result: Option<Result<CbRuleEquivalent, CbRuleCreationFailure>>,

    filter_list: String,
    filter_list_update_task: Option<Box<dyn Task>>,
    engine: adblock::Engine,
    metadata: adblock::lists::FilterListMetadata,

    network_url: String,
    network_source_url: String,
    network_request_type: String,
    network_result: Option<Result<adblock::blocker::BlockerResult, adblock::request::RequestError>>,

    cosmetic_url: String,
    cosmetic_result: Option<adblock::cosmetic_filter_cache::UrlSpecificResources>,
}

enum Msg {
    UpdateFilter(String),
    UpdateFilterList(String),
    FilterListTimeout,
    UpdateNetworkUrl(String),
    UpdateNetworkSourceUrl(String),
    UpdateNetworkRequestType(String),
    UpdateCosmeticUrl(String),
    DownloadDat,
}

const FILTER_LIST_UPDATE_DEBOUNCE_MS: u64 = 1200;

impl Component for Model {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,

            filter: "".into(),
            parse_result: Err(FilterParseError::Empty),
            cb_result: None,

            filter_list: "".into(),
            filter_list_update_task: None,
            engine: adblock::Engine::new(false),
            metadata: adblock::lists::FilterListMetadata::default(),

            network_url: String::new(),
            network_source_url: String::new(),
            network_request_type: String::new(),
            network_result: None,

            cosmetic_url: String::new(),
            cosmetic_result: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UpdateFilter(new_value) => {
                self.filter = new_value;
                self.parse_result = parse_filter(&self.filter, true, ParseOptions { rule_types: RuleTypes::All, format: FilterFormat::Standard, permissions: PermissionMask::from_bits(0) });
                self.cb_result = parse_filter(&self.filter, true, ParseOptions { rule_types: RuleTypes::All, format: FilterFormat::Standard, permissions: PermissionMask::from_bits(0) }).ok().map(|r| r.try_into());
            }
            Msg::UpdateFilterList(new_value) => {
                self.filter_list = new_value;

                // Cancel any previous timer
                self.filter_list_update_task.take();

                // Remove any existing block result
                self.network_result.take();

                // Start a new 3 second timeout
                self.filter_list_update_task = Some(Box::new(TimeoutService::spawn(
                    std::time::Duration::from_millis(FILTER_LIST_UPDATE_DEBOUNCE_MS),
                    self.link.callback(|_| Msg::FilterListTimeout),
                )));
            }
            Msg::FilterListTimeout => {
                let mut filter_set = adblock::lists::FilterSet::new(true);
                self.metadata = filter_set.add_filter_list(&self.filter_list, ParseOptions::default());
                self.engine = adblock::Engine::from_filter_set(filter_set, false);
                self.check_network_urls();
            }
            Msg::UpdateNetworkUrl(new_value) => {
                self.network_url = new_value;
                self.check_network_urls();
            }
            Msg::UpdateNetworkSourceUrl(new_value) => {
                self.network_source_url = new_value;
                self.check_network_urls();
            }
            Msg::UpdateNetworkRequestType(new_value) => {
                self.network_request_type = new_value;
                self.check_network_urls();
            }
            Msg::UpdateCosmeticUrl(new_value) => {
                self.cosmetic_url = new_value;
                self.cosmetic_result = Some(self.engine.url_cosmetic_resources(&self.cosmetic_url));
            }
            Msg::DownloadDat => {
                let data = self.engine.serialize_raw().unwrap();
                util::save_bin_file("rs-ABPFilterParserData.dat", &data[..]);
            }
        }
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <>
                <h1><code>{"adblock-rust"}</code>{" Dashboard"}</h1>
                <a href="https://github.com/brave-experiments/adblock-rust-dashboard"><p>{"View source on GitHub"}</p></a>
                <div>
                    <h2>{"Parse a single filter"}</h2>
                    <input type="text" value=self.filter.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateFilter(e.value))/>

                    { match &self.parse_result {
                        Ok(ParsedFilter::Network(filter)) => Self::view_network_filter(filter),
                        Ok(ParsedFilter::Cosmetic(filter)) => Self::view_cosmetic_filter(filter),
                        Err(FilterParseError::Network(e)) => html! { <p>{"Error parsing network filter: "}<code class="error">{format!("{}", e)}</code></p> },
                        Err(FilterParseError::Cosmetic(e)) => html! { <p>{"Error parsing cosmetic filter: "}<code class="error">{format!("{}", e)}</code></p> },
                        Err(FilterParseError::Unsupported) => html! { <p>{"Unsupported filter"}</p> },
                        Err(FilterParseError::Empty) => html! { <p></p> },
                    } }

                    { if let Some(cb_result) = &self.cb_result {
                        html! {
                            <>
                                <h4>{"Content blocking syntax equivalent "}<a href="https://developer.apple.com/documentation/safariservices/creating_a_content_blocker">{"?"}</a></h4>
                                { match cb_result {
                                    Ok(CbRuleEquivalent::SingleRule(rule)) => Self::view_cb_rule(rule),
                                    Ok(CbRuleEquivalent::SplitDocument(rule1, rule2)) => html! {
                                        <>
                                            {Self::view_cb_rule(rule1)}
                                            {Self::view_cb_rule(rule2)}
                                        </>
                                    },
                                    Err(e) => html! { <p>{"Couldn't convert to content blocking syntax: "}<code class="error">{format!("{:?}", e)}</code></p> },
                                } }
                            </>
                        }
                    } else {
                        html! { <></> }
                    } }
                </div>
                <div>
                    <h2>{"Test a list"}</h2>
                    <h3>{"List contents"}</h3>
                    <textarea value=self.filter_list.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateFilterList(e.value))/>
                    { Self::view_list_metadata(&self.metadata) }
                    <h3>{"Check a network request"}</h3>
                    <h4>{"Request URL"}</h4>
                    <input type="text" value=self.network_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkUrl(e.value))/>
                    <h4>{"Source URL"}</h4>
                    <input type="text" value=self.network_source_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkSourceUrl(e.value))/>
                    <h4>{"Request type"}</h4>
                    <input type="text" value=self.network_request_type.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkRequestType(e.value))/>
                    {
                        match self.network_result.as_ref() {
                            Some(Ok(blocker_result)) => html! {
                                <>
                                    <p><code>{format!("{:?}", blocker_result)}</code></p>
                                    <p><i>{"Note: redirects will not show up, as none have been loaded"}</i></p>
                                </>
                            },
                            Some(Err(request_error)) => html! {
                                <>
                                    <p>{"Error parsing request: "}<code class="error">{format!("{}", request_error)}</code></p>
                                </>
                            },
                            None => html! { <p></p> },
                        }
                    }
                    <h3>{"Check cosmetic resources"}</h3>
                    <h4>{"Source URL"}</h4>
                    <input type="text" value=self.cosmetic_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateCosmeticUrl(e.value))/>
                    {
                        if let Some(cosmetic_result) = self.cosmetic_result.as_ref() {
                            html! {
                                <>
                                    <p><code>{format!("{:?}", cosmetic_result)}</code></p>
                                    <p><i>{"Note: scriptlets will not show up, as none have been loaded"}</i></p>
                                </>
                            }
                        } else {
                            html! { <p></p> }
                        }
                    }
                    <h3>{"Download the serialized DAT"}</h3>
                    <button onclick=self.link.callback(|_e: MouseEvent| Msg::DownloadDat)>{"Download"}</button>
                </div>
            </>
        }
    }
}

impl Model {
    fn view_network_filter(filter: &adblock::filters::network::NetworkFilter) -> Html {
        html! {
            <>
                <h4>{"Network Filter"}</h4>
                <p><code>{ format!("{:?}", filter) }</code></p>
            </>
        }
    }
    fn view_cosmetic_filter(filter: &adblock::filters::cosmetic::CosmeticFilter) -> Html {
        html! {
            <>
                <h4>{"Cosmetic Filter"}</h4>
                <p><code>{ format!("{:?}", filter) }</code></p>
            </>
        }
    }
    fn view_cb_rule(filter: &adblock::content_blocking::CbRule) -> Html {
        let cb = serde_json::to_string(filter).unwrap();
        html! {
            <>
                <p><code>{ format!("{}", cb) }</code></p>
            </>
        }
    }
    fn check_network_urls(&mut self) {
        self.network_result = if self.network_url.is_empty() && self.network_source_url.is_empty() && self.network_request_type.is_empty() {
            None
        } else {
            Some(
                adblock::request::Request::new(&self.network_url, &self.network_source_url, &self.network_request_type)
                    .map(|request| self.engine.check_network_request(&request))
            )
        }
    }

    fn view_list_metadata(metadata: &adblock::lists::FilterListMetadata) -> Html {
        fn view_link(name: &str, field: &Option<String>) -> Html {
            html! {
                if let Some(link) = field {
                    html! { <div><span>{format!("{}: ", name)}<a href={format!("{}", link)}><code>{format!("{}", link)}</code></a></span></div> }
                } else {
                    html! { <></> }
                }
            }
        }
        html! {
            <>
                {
                    if let Some(title) = &metadata.title {
                        html! { <div><span>{"Title: "}<code>{format!("{}", title)}</code></span></div> }
                    } else {
                        html! { <></> }
                    }
                }
                { view_link("Homepage", &metadata.homepage) }
                {
                    if let Some(expires) = &metadata.expires {
                        html! { <div><span>{"Expires: "}<code>{format!("{:?}", expires)}</code></span></div> }
                    } else {
                        html! { <></> }
                    }
                }
                { view_link("Redirect", &metadata.redirect) }
            </>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    App::<Model>::new().mount_to_body();
}
