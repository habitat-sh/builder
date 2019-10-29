// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import 'whatwg-fetch';
import config from '../config';
import {
  setCurrentUsername,
  addNotification,
  fetchMyOrigins,
  fetchMyOriginInvitations,
  fetchProfile,
  setPrivileges,
  signingIn,
  signInFailed
} from './index';
import { DANGER } from './notifications';
import { setBldrSessionToken } from './sessions';
import { Browser } from '../browser';
import { OAuthProvider } from '../oauth-providers';

const uuid = require('uuid').v4;
const authenticateEndpoint = 'v1/authenticate';

export const LOAD_OAUTH_STATE = 'LOAD_OAUTH_STATE';
export const POPULATE_GITHUB_INSTALLATIONS = 'POPULATE_GITHUB_INSTALLATIONS';
export const POPULATE_GITHUB_REPOSITORIES = 'POPULATE_GITHUB_REPOSITORIES';
export const SET_OAUTH_PROVIDER = 'SET_OAUTH_PROVIDER';
export const SET_OAUTH_STATE = 'SET_OAUTH_STATE';
export const SET_OAUTH_TOKEN = 'SET_OAUTH_TOKEN';

export function authenticate(oauthToken: string, bldrToken: string) {
  return (dispatch, getState) => {

    if (oauthToken) {
      dispatch(setOAuthToken(oauthToken));
    }

    if (bldrToken) {
      dispatch(setBldrSessionToken(bldrToken));
      dispatch(fetchMyOrigins(bldrToken));
      dispatch(fetchMyOriginInvitations(bldrToken));
      dispatch(fetchProfile(bldrToken));
    }
  };
}

export function exchangeOAuthCode(code: string, state: string) {

  return (dispatch, getState) => {

    if (getState().oauth.provider.useState && state !== getState().oauth.state) {
      dispatch(signInFailed());
      return;
    }

    dispatch(signingIn(true));

    fetch(`${authenticateEndpoint}/${code}`).then(response => {
      return response.json();
    })
      .then(data => {
        dispatch(signingIn(false));

        if (data.oauth_token && data.token) {
          dispatch(authenticate(data.oauth_token, data.token));
          dispatch(setCurrentUsername(data.login));
          dispatch(setPrivileges(data.flags));
        } else {
          dispatch(signInFailed());
          dispatch(addNotification({
            title: 'Authentication Failed',
            body: `[err=${data.code}] ${data.msg}`,
            type: DANGER
          }));
        }
      })
      .catch(error => {
        dispatch(signingIn(false));
        dispatch(signInFailed());
        dispatch(addNotification({
          title: 'Authentication Failed',
          body: 'Unable to retrieve OAuth token.',
          type: DANGER
        }));
      });
  };
}

export function loadOAuthState() {
  return {
    type: LOAD_OAUTH_STATE,
    payload: {
      token: Browser.getCookie('oauthToken'),
      state: Browser.getCookie('oauthState')
    },
  };
}

export function removeSession() {
  return dispatch => {
    Browser.removeCookie('oauthState');
    Browser.removeCookie('oauthToken');
    Browser.removeCookie('bldrSessionToken');
  };
}

export function setOAuthState() {
  let payload = Browser.getCookie('oauthState') || uuid();
  Browser.setCookie('oauthState', payload);

  return {
    type: SET_OAUTH_STATE,
    payload
  };
}

export function loadOAuthProvider() {
  return (dispatch, getState) => {
    dispatch(setOAuthState());
    dispatch(setOAuthProvider(
      OAuthProvider.fromConfig(
        config.oauth_provider,
        config.oauth_client_id,
        config.oauth_authorize_url,
        config.oauth_redirect_url,
        config.oauth_signup_url,
        getState().oauth.state
      )
    ));
  };
}

function setOAuthProvider(payload) {
  return {
    type: SET_OAUTH_PROVIDER,
    payload
  };
}

export function setOAuthToken(payload) {
  Browser.setCookie('oauthToken', payload);

  return {
    type: SET_OAUTH_TOKEN,
    payload
  };
}
