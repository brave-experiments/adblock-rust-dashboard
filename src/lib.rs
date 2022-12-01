#![recursion_limit="1024"]

use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::services::{Task, TimeoutService};

use adblock::lists::{parse_filter, FilterFormat, FilterParseError, ParsedFilter, ParseOptions, RuleTypes};

mod util;

struct Model {
    link: ComponentLink<Self>,

    filter: String,
    parse_result: Result<ParsedFilter, FilterParseError>,

    filter_list: String,
    filter_list_update_task: Option<Box<dyn Task>>,
    engine: adblock::engine::Engine,

    network_url: String,
    network_source_url: String,
    network_request_type: String,
    network_result: Option<adblock::blocker::BlockerResult>,

    cosmetic_url: String,
    cosmetic_result: Option<adblock::cosmetic_filter_cache::UrlSpecificResources>,

    download_legacy_format: bool,
}

enum Msg {
    UpdateFilter(String),
    UpdateFilterList(String),
    FilterListTimeout,
    UpdateNetworkUrl(String),
    UpdateNetworkSourceUrl(String),
    UpdateNetworkRequestType(String),
    UpdateCosmeticUrl(String),
    ToggleDownloadFormat,
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

            filter_list: "".into(),
            filter_list_update_task: None,
            engine: adblock::engine::Engine::new(false),

            network_url: String::new(),
            network_source_url: String::new(),
            network_request_type: String::new(),
            network_result: None,

            cosmetic_url: String::new(),
            cosmetic_result: None,

            download_legacy_format: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UpdateFilter(new_value) => {
                self.filter = new_value;
                let result = parse_filter(&self.filter, true, ParseOptions { rule_types: RuleTypes::All, format: FilterFormat::Standard });
                self.parse_result = result;
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
                filter_set.add_filter_list(&self.filter_list, ParseOptions::default());
                self.engine = adblock::engine::Engine::from_filter_set(filter_set, false);
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
            Msg::ToggleDownloadFormat => {
                self.download_legacy_format = !self.download_legacy_format;
            }
            Msg::DownloadDat => {
                let data = if self.download_legacy_format {
                    self.engine.serialize_compressed().unwrap()
                } else {
                    self.engine.serialize_raw().unwrap()
                };
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
                        Err(FilterParseError::Network(e)) => html! { <p>{format!("Error parsing network filter: {:?}", e)}</p> },
                        Err(FilterParseError::Cosmetic(e)) => html! { <p>{format!("Error parsing cosmetic filter: {:?}", e)}</p> },
                        Err(FilterParseError::Unsupported) => html! { <p>{"Unsupported filter"}</p> },
                        Err(FilterParseError::Empty) => html! { <p></p> },
                    } }
                </div>
                <div>
                    <h2>{"Test a list"}</h2>
                    <h3>{"List contents"}</h3>
                    <textarea value=self.filter_list.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateFilterList(e.value))/>
                    <h3>{"Check a network request"}</h3>
                    <h4>{"Request URL"}</h4>
                    <input type="text" value=self.network_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkUrl(e.value))/>
                    <h4>{"Source URL"}</h4>
                    <input type="text" value=self.network_source_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkSourceUrl(e.value))/>
                    <h4>{"Request type"}</h4>
                    <input type="text" value=self.network_request_type.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateNetworkRequestType(e.value))/>
                    {
                        if let Some(blocker_result) = self.network_result.as_ref() {
                            if let Some(error) = blocker_result.error.as_ref() {
                                html! { <p>{format!("Error: {}", error)}</p> }
                            } else {
                                html! {
                                    <>
                                        <p>{format!("{:?}", blocker_result)}</p>
                                        <p><i>{"Note: redirects will not show up, as none have been loaded"}</i></p>
                                    </>
                                }
                            }
                        } else {
                            html! { <p></p> }
                        }
                    }
                    <h3>{"Check cosmetic resources"}</h3>
                    <h4>{"Source URL"}</h4>
                    <input type="text" value=self.cosmetic_url.clone() oninput=self.link.callback(|e: InputData| Msg::UpdateCosmeticUrl(e.value))/>
                    {
                        if let Some(cosmetic_result) = self.cosmetic_result.as_ref() {
                            html! {
                                <>
                                    <p>{format!("{:?}", cosmetic_result)}</p>
                                    <p><i>{"Note: scriptlets will not show up, as none have been loaded"}</i></p>
                                </>
                            }
                        } else {
                            html! { <p></p> }
                        }
                    }
                    <h3>{"Download the serialized DAT"}</h3>
                    <input id="download_legacy_format" type="checkbox" checked=self.download_legacy_format onchange=self.link.callback(|_e: ChangeData| Msg::ToggleDownloadFormat)/>
                    <label for="download_legacy_format">{"Download legacy (compressed) format"}</label>
                    <br/>
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
                <p>{ format!("{:?}", filter) }</p>
            </>
        }
    }
    fn view_cosmetic_filter(filter: &adblock::filters::cosmetic::CosmeticFilter) -> Html {
        html! {
            <>
                <h4>{"Cosmetic Filter"}</h4>
                <p>{ format!("{:?}", filter) }</p>
            </>
        }
    }
    fn check_network_urls(&mut self) {
        self.network_result = Some(self.engine.check_network_urls(&self.network_url, &self.network_source_url, &self.network_request_type));
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    App::<Model>::new().mount_to_body();
}
