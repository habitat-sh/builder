// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

import { authenticate, removeSession, loadOAuthProvider, loadBldrSessionState, loadOAuthState, requestRoute, resetAppState } from './index';
import { addNotification, SUCCESS, DANGER } from './notifications';
import { BuilderApiClient } from '../client/builder-api';
import { Browser } from '../browser';

export const CLEAR_ACCESS_TOKENS = 'CLEAR_ACCESS_TOKENS';
export const CLEAR_NEW_ACCESS_TOKEN = 'CLEAR_NEW_ACCESS_TOKEN';
export const POPULATE_ACCESS_TOKENS = 'POPULATE_ACCESS_TOKENS';
export const POPULATE_NEW_ACCESS_TOKEN = 'POPULATE_NEW_ACCESS_TOKEN';
export const POPULATE_PROFILE = 'POPULATE_PROFILE';
export const SET_LOADING_ACCESS_TOKENS = 'SET_LOADING_ACCESS_TOKENS';
export const SET_DELETING_ACCESS_TOKEN = 'SET_DELETING_ACCESS_TOKEN';
export const SET_GENERATING_ACCESS_TOKEN = 'SET_GENERATING_ACCESS_TOKEN';
export const SET_PRIVILEGES = 'SET_PRIVILEGES';
export const SET_CURRENT_USERNAME = 'SET_CURRENT_USERNAME';
export const SIGN_IN_FAILED = 'SIGN_IN_FAILED';
export const SIGNING_IN = 'SIGNING_IN';
export const TOGGLE_USER_NAV_MENU = 'TOGGLE_USER_NAV_MENU';

export function fetchProfile(token: string) {
  return dispatch => {
    new BuilderApiClient(token).getProfile()
      .then(data => {
        dispatch(populateProfile(data));
        notifySegment(data);
      })
      .catch(err => { });
  };
}

export function fetchAccessTokens(token: string) {
  return dispatch => {
    dispatch(setLoadingAccessTokens(true));

    new BuilderApiClient(token).getAccessTokens()
      .then(data => {
        dispatch(populateAccessTokens(data));
        dispatch(setLoadingAccessTokens(false));
        notifySegment(data);
      })
      .catch(err => {
        dispatch(setLoadingAccessTokens(false));
      });
  };
}

export function generateAccessToken(token: string) {
  return dispatch => {
    dispatch(clearNewAccessToken());
    dispatch(setGeneratingAccessToken(true));

    new BuilderApiClient(token).generateAccessToken()
      .then(data => {
        dispatch(populateNewAccessToken(data));
        dispatch(setGeneratingAccessToken(false));
      })
      .catch(err => {
        dispatch(addNotification({
          title: 'Error generating access token',
          body: `${err.message}`,
          type: DANGER
        }));
        dispatch(setGeneratingAccessToken(false));
      });
  };
}

export function deleteAccessToken(id: string, token: string) {
  return dispatch => {
    dispatch(setDeletingAccessToken(true));

    new BuilderApiClient(token).deleteAccessToken(id)
      .then(data => {
        dispatch(addNotification({
          title: 'Personal access token deleted',
          type: SUCCESS
        }));
        dispatch(setDeletingAccessToken(false));
      })
      .catch(err => {
        dispatch(setDeletingAccessToken(false));
      });
  };
}

export function identifyUser() {
  return (dispatch, getState) => {
    dispatch(loadBldrSessionState());
    dispatch(loadOAuthState());

    const oauthToken = getState().oauth.token;
    const bldrToken = getState().session.token;

    if (oauthToken && bldrToken) {
      dispatch(authenticate(oauthToken, bldrToken));
    }
  };
}

export function saveProfile(profile: any, token: string) {
  return dispatch => {
    new BuilderApiClient(token).saveProfile(profile)
      .then(() => {
        dispatch(addNotification({
          title: 'Profile saved',
          type: SUCCESS
        }));
        dispatch(fetchProfile(token));
      })
      .catch(err => {
        dispatch(addNotification({
          title: 'Error saving profile',
          body: `${err.message}`,
          type: DANGER
        }));
      });
  };
}

export function clearAccessTokens() {
  return {
    type: CLEAR_ACCESS_TOKENS
  };
}

export function clearNewAccessToken() {
  return {
    type: CLEAR_NEW_ACCESS_TOKEN
  };
}

function populateAccessTokens(payload) {
  return {
    type: POPULATE_ACCESS_TOKENS,
    payload
  };
}

function populateNewAccessToken(payload) {
  return {
    type: POPULATE_NEW_ACCESS_TOKEN,
    payload
  };
}

export function setLoadingAccessTokens(payload) {
  return {
    type: SET_LOADING_ACCESS_TOKENS,
    payload
  };
}

export function setGeneratingAccessToken(payload) {
  return {
    type: SET_GENERATING_ACCESS_TOKEN,
    payload
  };
}

export function setDeletingAccessToken(payload) {
  return {
    type: SET_DELETING_ACCESS_TOKEN,
    payload
  };
}

function notifySegment(data: any) {
  const segment = window['analytics'];

  if (segment && typeof segment.identify === 'function') {
    segment.identify(data.id, { email: data.email, name: data.name });
  }
}

function populateProfile(payload) {
  return {
    type: POPULATE_PROFILE,
    payload
  };
}

export function setCurrentUsername(payload) {
  return {
    type: SET_CURRENT_USERNAME,
    payload
  };
}

export function toggleUserNavMenu() {
  return {
    type: TOGGLE_USER_NAV_MENU
  };
}

export function setPrivileges(payload) {
  return {
    type: SET_PRIVILEGES,
    payload
  };
}

export function signInFailed() {
  return {
    type: SIGN_IN_FAILED
  };
}

export function signingIn(payload) {
  return {
    type: SIGNING_IN,
    payload
  };
}

export function signOut(redirectToSignIn: boolean, pathAfterSignIn?: string) {
  return (dispatch, getState) => {

    if (getState().session.token) {
      dispatch(removeSession());
      dispatch(resetAppState());
    }

    if (pathAfterSignIn) {
      const key = 'redirectPath';

      if (!Browser.getCookie(key)) {
        Browser.setCookie(key, pathAfterSignIn);
      }
    }

    dispatch(loadOAuthProvider());

    if (redirectToSignIn) {
      dispatch(requestRoute(['/sign-in']));
    }
  };
}
