use actix_web::{actix::Handler, error, Error};
use server::db::DbExecutor;
use server::models::account::{
    Account, CreateAccount, FindOrCreateAccount, GetAccount, GetAccountById, UpdateAccount,
};
use std::ops::Deref;

impl Handler<GetAccount> for DbExecutor {
    type Result = Result<Account, Error>;

    fn handle(&mut self, account: GetAccount, _: &mut Self::Context) -> Self::Result {
        Account::get(account, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error fetching account"))
    }
}

impl Handler<GetAccountById> for DbExecutor {
    type Result = Result<Account, Error>;

    fn handle(&mut self, id: GetAccountById, _: &mut Self::Context) -> Self::Result {
        Account::get_by_id(id, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error fetching account by ID"))
    }
}

impl Handler<CreateAccount> for DbExecutor {
    type Result = Result<Account, Error>;

    fn handle(&mut self, account: CreateAccount, _: &mut Self::Context) -> Self::Result {
        Account::create(account, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error creating account"))
    }
}

impl Handler<UpdateAccount> for DbExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, account: UpdateAccount, _: &mut Self::Context) -> Self::Result {
        Account::update(account, self.get_conn()?.deref())
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error updating account"))
    }
}

impl Handler<FindOrCreateAccount> for DbExecutor {
    type Result = Result<Account, Error>;

    fn handle(&mut self, account: FindOrCreateAccount, _: &mut Self::Context) -> Self::Result {
        Account::find_or_create(account, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error on FetchOrCreate account"))
    }
}
