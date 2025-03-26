#![recursion_limit="1024"]

use adblock::resources::PermissionMask;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

use adblock::lists::{parse_filter, FilterFormat, FilterParseError, ParsedFilter, ParseOptions, RuleTypes};
use adblock::content_blocking::{CbRuleEquivalent, CbRuleCreationFailure};

mod util;

struct Model {
    filter: String,
    parse_result: Result<ParsedFilter, FilterParseError>,
    cb_result: Option<Result<CbRuleEquivalent, CbRuleCreationFailure>>,

    filter_list: String,
    filter_list_update_task: Option<gloo_timers::callback::Timeout>,
    engine: adblock::Engine,
    metadata: adblock::lists::FilterListMetadata,

    network_url: String,
    network_source_url: String,
    network_request_type: String,
    network_result: Option<Result<adblock::blocker::BlockerResult, adblock::request::RequestError>>,

    cosmetic_url: String,
    cosmetic_result: Option<adblock::cosmetic_filter_cache::UrlSpecificResources>,

    resources: Vec<adblock::resources::Resource>,
}

enum Msg {
    UpdateFilter(String),
    UpdateFilterList(String),
    FilterListTimeout,
    UpdateNetworkUrl(String),
    UpdateNetworkSourceUrl(String),
    UpdateNetworkRequestType(String),
    UpdateCosmeticUrl(String),
    LoadResourcesJson(String),
    DownloadDat,
}

const FILTER_LIST_UPDATE_DEBOUNCE_MS: u32 = 1200;

impl Component for Model {
    type Message = Msg;
    type Properties = ();
    fn create(_ctx: &Context<Self>) -> Self {
        Self {
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

            resources: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
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

                let link = ctx.link().clone();

                // Start a new 3 second timeout
                self.filter_list_update_task = Some(gloo_timers::callback::Timeout::new(
                    FILTER_LIST_UPDATE_DEBOUNCE_MS,
                    move || { link.send_message(Msg::FilterListTimeout); }
                ));
            }
            Msg::FilterListTimeout => {
                let mut filter_set = adblock::lists::FilterSet::new(true);
                self.metadata = filter_set.add_filter_list(&self.filter_list, ParseOptions::default());
                self.engine = adblock::Engine::from_filter_set(filter_set, false);
                self.engine.use_resources(self.resources.iter().map(|r| r.clone()));
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
            Msg::LoadResourcesJson(new_value) => {
                let resources: Vec<_> = serde_json::from_str(&new_value).unwrap();
                self.resources = resources;
                self.engine.use_resources(self.resources.iter().map(|r| r.clone()));
            }
            Msg::DownloadDat => {
                let data = self.engine.serialize_raw().unwrap();
                util::save_bin_file("rs-ABPFilterParserData.dat", &data[..]);
            }
        }
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <h1><code>{"adblock-rust"}</code>{" Dashboard"}</h1>
                <a href="https://github.com/brave-experiments/adblock-rust-dashboard"><p>{"View source on GitHub"}</p></a>
                <div>
                    <h2>{"Parse a single filter"}</h2>
                    <input type="text" value={self.filter.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateFilter(e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>

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
                    <textarea value={self.filter_list.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateFilterList(e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap().value()))}/>
                    <input type="file" accept=".json,application/json" id="load_resources_json" oninput={
                        let link = ctx.link().clone();
                        move |e: InputEvent| {
                            let link = link.clone();
                            let input_element = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                            if let Some(file) = input_element.files().unwrap().item(0) {
                                unsafe {
                                    read_file_text_and_then(&file, move |text| {
                                        let link = link.clone();
                                        link.send_message(Msg::LoadResourcesJson(text));
                                    });
                                }
                            }
                            input_element.set_value("");
                        }
                    }/>
                    <div>
                        <label for="load_resources_json"><span>{"Load "}</span><code>{"resources.json"}</code></label>
                        <i>{
                            if self.resources.len() > 0 {
                                format!(" {} resources loaded", self.resources.len())
                            } else {
                                " No resources loaded".to_string()
                            }
                        }</i>
                    </div>
                    { Self::view_list_metadata(&self.metadata) }
                    <h3>{"Check a network request"}</h3>
                    <h4>{"Request URL"}</h4>
                    <input type="text" value={self.network_url.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateNetworkUrl(e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>
                    <h4>{"Source URL"}</h4>
                    <input type="text" value={self.network_source_url.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateNetworkSourceUrl(e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>
                    <h4>{"Request type"}</h4>
                    <input type="text" value={self.network_request_type.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateNetworkRequestType(e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>
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
                    <input type="text" value={self.cosmetic_url.clone()} oninput={ctx.link().callback(|e: InputEvent| Msg::UpdateCosmeticUrl(e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>
                    {
                        if let Some(cosmetic_result) = self.cosmetic_result.as_ref() {
                            let resources_disclaimer = if self.resources.is_empty() {
                                html! {
                                    <p><i>{"Note: scriptlets will not show up, as none have been loaded"}</i></p>
                                }
                            } else {
                                html! {}
                            };
                            html! {
                                <>
                                    <p><code>{format!("{:?}", cosmetic_result)}</code></p>
                                    {resources_disclaimer}
                                </>
                            }
                        } else {
                            html! { <p></p> }
                        }
                    }
                    <h3>{"Download the serialized DAT"}</h3>
                    <button onclick={ctx.link().callback(|_e: MouseEvent| Msg::DownloadDat)}>{"Download"}</button>
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
            if let Some(link) = field {
                html! { <div><span>{format!("{}: ", name)}<a href={format!("{}", link)}><code>{format!("{}", link)}</code></a></span></div> }
            } else {
                html! { <></> }
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

/// Reads a file and then executes a closure on the text contents using `FileReader`.
unsafe fn read_file_text_and_then(file: &web_sys::File, closure: impl FnOnce(String) + 'static) {
    fn onload_helper(e: ProgressEvent, closure: impl FnOnce(String)) {
        let text = e.target().unwrap().dyn_into::<web_sys::FileReader>().unwrap().result().unwrap().as_string().unwrap();
        closure(text);
    }

    let filereader = web_sys::FileReader::new().unwrap();
    let closure = wasm_bindgen::closure::Closure::once(move |e: ProgressEvent| {
        onload_helper(e, closure);
    }).into_js_value().dyn_into::<web_sys::js_sys::Function>().unwrap();
    filereader.set_onload(Some(&closure));

    filereader.read_as_text(file).unwrap();
}

#[wasm_bindgen(start)]
pub fn run_app() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    yew::Renderer::<Model>::new().render();
}
