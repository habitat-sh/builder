use crate::{db::models::channel::{AuditPackage,
                                  AuditPackageEvent,
                                  ListEvents},
            server::{authorize::authorize_session,
                     error::{Error,
                             Result},
                     framework::headers,
                     helpers::{self,
                               req_state,
                               DateRange,
                               Pagination,
                               SearchQuery,
                               ToChannel},
                     AppState}};
use actix_web::{http,
                web::{self,
                      Data,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use builder_core::http_client::{HttpClient,
                                USER_AGENT_BLDR};

pub struct Events {}

impl Events {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/events", web::get().to(get_events))
           .route("/depot/events/saas", web::get().to(get_events_from_saas));
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_events(req: HttpRequest,
                    pagination: Query<Pagination>,
                    channel: Query<ToChannel>,
                    date_range: Query<DateRange>,
                    search_query: Query<SearchQuery>)
                    -> HttpResponse {
    match do_get_events(&req, &pagination, &channel, &date_range, &search_query) {
        Ok((events, count)) => postprocess_event_list(&req, &events, count, &pagination),
        Err(err) => {
            error!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_events_from_saas(req: HttpRequest,
                              pagination: Query<Pagination>,
                              channel: Query<ToChannel>,
                              date_range: Query<DateRange>,
                              search_query: Query<SearchQuery>,
                              state: Data<AppState>)
                              -> HttpResponse {
    let bldr_url = &state.config.api.saas_bldr_url;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(USER_AGENT_BLDR.0.clone(), USER_AGENT_BLDR.1.clone());
    if req.headers().contains_key(http::header::AUTHORIZATION) {
        headers.insert(http::header::AUTHORIZATION,
                       req.headers()
                          .get(http::header::AUTHORIZATION)
                          .unwrap()
                          .clone());
    }

    let http_client = match HttpClient::new(bldr_url, headers) {
        Ok(client) => client,
        Err(err) => {
            debug!("HttpClient Error: {:?}", err);
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };

    // We are expecting dates in YYYY-MM-DD format
    let from_date = date_range.from_date.date().format("%Y-%m-%d").to_string();
    let to_date = date_range.to_date.date().format("%Y-%m-%d").to_string();
    let url = format!("{}/v1/depot/events?range={}&channel={}&from_date={}&to_date={}&query={}",
                      bldr_url,
                      pagination.range,
                      channel.channel,
                      from_date,
                      to_date,
                      search_query.query);
    debug!("SaaS Url: {}", url);
    match http_client.get(&url)
                     .send()
                     .await
                     .map_err(Error::HttpClient)
    {
        Ok(response) => {
            match response.text().await {
                Ok(body) => {
                    let mut http_response = HttpResponse::Ok();

                    http_response.header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
                                 .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                                 .body(body)
                }
                Err(err) => {
                    let msg = format!("Error getting response text: {:?}", err);
                    debug!("{}", msg);
                    HttpResponse::InternalServerError().body(msg)
                }
            }
        }
        Err(err) => {
            let msg = format!("Error sending request: {:?}", err);
            debug!("{}", msg);
            HttpResponse::InternalServerError().body(msg)
        }
    }
}

fn do_get_events(req: &HttpRequest,
                 pagination: &Query<Pagination>,
                 channel: &Query<ToChannel>,
                 date_range: &Query<DateRange>,
                 search_query: &Query<SearchQuery>)
                 -> Result<(Vec<AuditPackageEvent>, i64)> {
    let opt_session_id = match authorize_session(req, None, None) {
        Ok(session) => Some(session.get_id() as i64),
        Err(_) => None,
    };
    let (page, per_page) = helpers::extract_pagination_in_pages(pagination);

    let conn = req_state(req).db.get_conn().map_err(Error::DbError)?;
    let decoded_query =
        match percent_encoding::percent_decode(search_query.query.as_bytes()).decode_utf8() {
            Ok(q) => q.to_string().trim_end_matches('/').replace("/", " & "),
            Err(err) => {
                error!("{}", err);
                return Err(Error::Unprocessable);
            }
        };

    let el = ListEvents { page:       page as i64,
                          limit:      per_page as i64,
                          account_id: opt_session_id,
                          channel:    channel.channel.trim().to_string(),
                          from_date:  date_range.from_date,
                          to_date:    date_range.to_date,
                          query:      decoded_query, };
    match AuditPackage::list(el, &*conn).map_err(Error::DieselError) {
        Ok((packages, count)) => {
            let pkg_events: Vec<AuditPackageEvent> =
                packages.into_iter().map(|p| p.into()).collect();

            Ok((pkg_events, count))
        }
        Err(e) => Err(e),
    }
}

pub fn postprocess_event_list(_req: &HttpRequest,
                              events: &[AuditPackageEvent],
                              count: i64,
                              pagination: &Query<Pagination>)
                              -> HttpResponse {
    let (start, _) = helpers::extract_pagination(pagination);
    let event_count = events.len() as isize;
    let stop = match event_count {
        0 => count,
        _ => (start + event_count - 1) as i64,
    };

    debug!("postprocessing event list, start: {}, stop: {}, total_count: {}",
           start, stop, count);

    let body =
        helpers::package_results_json(&events, count as isize, start as isize, stop as isize);

    let mut response = if count as isize > (stop as isize + 1) {
        HttpResponse::PartialContent()
    } else {
        HttpResponse::Ok()
    };

    response.header(http::header::CONTENT_TYPE, headers::APPLICATION_JSON)
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .body(body)
}
