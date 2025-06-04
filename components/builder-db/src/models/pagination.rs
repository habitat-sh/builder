use diesel::{pg::Pg,
             prelude::*,
             query_builder::*,
             query_dsl::methods::LoadQuery,
             sql_types::BigInt};

pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated { query: self,
                    per_page: DEFAULT_PER_PAGE,
                    page }
    }
}

const DEFAULT_PER_PAGE: i64 = 50;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query:    T,
    page:     i64,
    per_page: i64,
}

impl<T> Paginated<T> {
    pub fn per_page(self, per_page: i64) -> Self { Paginated { per_page, ..self } }

    pub fn load_and_count_pages<U>(self, conn: &mut PgConnection) -> QueryResult<(Vec<U>, i64)>
        where for<'a> Self: LoadQuery<'a, PgConnection, (U, i64)>
    {
        let per_page = self.per_page;
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.first().map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Ok((records, total_pages))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T> where T: QueryFragment<Pg>
{
    fn walk_ast<'query>(&'query self, mut out: AstPass<'_, 'query, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(&self.per_page.to_string());
        let offs = (self.page - 1) * self.per_page;
        out.push_sql(&offs.to_string());
        Ok(())
    }
}
