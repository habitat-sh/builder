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
  signInFailed,
  signOut
} from './index';
import { DANGER, WARNING } from './notifications';
import { setBldrSessionToken } from './sessions';
import { Browser } from '../browser';

const uuid = require('uuid').v4;
const authenticateEndpoint = `${config['habitat_api_url']}/v1/authenticate`;

export const LOAD_OAUTH_STATE = 'LOAD_OAUTH_STATE';
export const POPULATE_GITHUB_INSTALLATIONS = 'POPULATE_GITHUB_INSTALLATIONS';
export const POPULATE_GITHUB_REPOSITORIES = 'POPULATE_GITHUB_REPOSITORIES';
export const POPULATE_GITHUB_USER_DATA = 'POPULATE_GITHUB_USER_DATA';
export const SET_OAUTH_STATE = 'SET_OAUTH_STATE';
export const SET_OAUTH_TOKEN = 'SET_OAUTH_TOKEN';

export function authenticate(oauthToken: string, bldrToken: string) {
  return (dispatch, getState) => {

    if (oauthToken) {
      dispatch(setOAuthToken(oauthToken));

      if (config.oauth_provider === 'github') {
        fetch(`${config.github_api_url}/user?access_token=${oauthToken}`).then(response => {
          if (response.ok) {
            return response.json();
          } else {
            return response.json().then(error => { throw error; });
          }
        })
          .then(data => {
            dispatch(populateGitHubUserData(data));
            dispatch(setCurrentUsername(data.login));
          })
          .catch(error => {
            // We can assume an error from the response is a 401; anything
            // else is probably a transient failure on GitHub's end, which
            // we can expect to clear when we try to sign in again.
            //
            // When we get an unauthorized response, our token is no
            // longer valid, so sign out.
            dispatch(signOut(true, getState().router.route.url));
            dispatch(addNotification({
              title: 'Authorization Failed',
              body: 'Please sign in again.',
              type: WARNING,
            }));
          });
      }
      else if (config.oauth_provider === 'chef-automate') {
        debugger;
      }
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
    dispatch(setOAuthState());

    if (state === getState().oauth.state) {
      dispatch(signingIn(true));

      fetch(`${authenticateEndpoint}/${code}`).then(response => {
        return response.json();
      })
        .then(data => {
          dispatch(signingIn(false));

          if (data.oauth_token && data.token) {
            dispatch(authenticate(data.oauth_token, data.token));
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
    }
    else {
      dispatch(signInFailed());
    }
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

function populateGitHubUserData(payload) {
  return {
    type: POPULATE_GITHUB_USER_DATA,
    payload,
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

export function setOAuthToken(payload) {
  Browser.setCookie('oauthToken', payload);

  return {
    type: SET_OAUTH_TOKEN,
    payload
  };
}
