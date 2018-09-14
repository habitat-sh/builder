// use diesel::{self,sql_query,RunQueryDsl,QueryDsl,ExpressionMethods};
// use actix_web::{actix::Handler, error,Error};
// use chrono::Utc;
// use model::response::{ArticleListMsgs, ArticleMsgs, Msgs};
// use model::article::{Article, ArticleList, ArticleId, NewArticle, ArticleNew};
// use model::db::ConnDsl;

// impl Handler<ChannelList> for DbPool {
//     type Result = Result<ChannelListResp, Error>;

//     fn handle(&mut self, article_list: ChannelList, _: &mut Self::Context) -> Self::Result {
//         use share::schema::article::dsl::*;
//         let conn = &self.0.get().map_err(error::ErrorInternalServerError)?;
//         let articles = article.load::<Article>(conn).map_err(error::ErrorInternalServerError)?;
//         Ok(ChannelListResp {
//             status: 200,
//             message : "article_list result Success.".to_string(),
//             article_list: articles,
//         })
//     }
// }
