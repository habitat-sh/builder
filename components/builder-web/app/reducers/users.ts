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

import * as actionTypes from '../actions/index';
import initialState from '../initial-state';

export default function users(state = initialState['users'], action) {
  switch (action.type) {

    case actionTypes.CLEAR_ACCESS_TOKENS:
      return state.setIn(['current', 'accessTokens'], []);

    case actionTypes.CLEAR_NEW_ACCESS_TOKEN:
      return state.setIn(['current', 'newAccessToken'], undefined);

    case actionTypes.POPULATE_ACCESS_TOKENS:
      return state.setIn(['current', 'accessTokens'], action.payload.tokens);

    case actionTypes.POPULATE_NEW_ACCESS_TOKEN:
      return state.setIn(['current', 'newAccessToken'], action.payload);

    case actionTypes.POPULATE_PROFILE:
      return state.setIn(['current', 'profile'], action.payload);

    case actionTypes.SET_DELETING_ACCESS_TOKEN:
      return state.setIn(['current', 'ui', 'accessTokens', 'deleting'], action.payload);

    case actionTypes.SET_GENERATING_ACCESS_TOKEN:
      return state.setIn(['current', 'ui', 'accessTokens', 'generating'], action.payload);

    case actionTypes.SET_LOADING_ACCESS_TOKENS:
      return state.setIn(['current', 'ui', 'accessTokens', 'loading'], action.payload);

    case actionTypes.SET_PRIVILEGES:
      return state.setIn(['current', 'flags'], action.payload);

    case actionTypes.SET_CURRENT_USERNAME:
      return state.setIn(['current', 'username'], action.payload);

    case actionTypes.SIGN_IN_FAILED:
      return state.setIn(['current', 'failedSignIn'], true);

    case actionTypes.SIGNING_IN:
      return state.setIn(['current', 'isSigningIn'], action.payload);

    case actionTypes.TOGGLE_USER_NAV_MENU:
      return state.setIn(['current', 'isUserNavOpen'], !state.getIn(['current', 'isUserNavOpen']));

    case actionTypes.FETCH_LICENSE_KEY_BEGIN:
      return state
        .setIn(['current', 'license', 'licenseKey'], null)
        .setIn(['current', 'license', 'expirationDate'], null)
        .setIn(['current', 'license', 'licenseFetchInProgress'], true)
        .setIn(['current', 'license', 'fetchedLicenseMessage'], null)
        .setIn(['current', 'license', 'isValid'], null);

    case actionTypes.FETCH_LICENSE_KEY_SUCCESS:
      return state
        .setIn(['current', 'license', 'licenseKey'], action.payload.license_key || null)
        .setIn(['current', 'license', 'expirationDate'], action.payload.expiration_date || null)
        .setIn(['current', 'license', 'licenseFetchInProgress'], false)
        .setIn(['current', 'license', 'isValid'], action.payload.isValid);

    case actionTypes.SAVE_LICENSE_KEY_BEGIN:
      return state
        .setIn(['current', 'license', 'saveLicenseKeyErrorMessage'], null);

    case actionTypes.SAVE_LICENSE_KEY_SUCCESS:
      return state
        .setIn(['current', 'license', 'saveLicenseKeyErrorMessage'], null);

    case actionTypes.SAVE_LICENSE_KEY_FAILED:
      return state
        .setIn(['current', 'license', 'saveLicenseKeyErrorMessage'], action.payload.errorMessage);

    case actionTypes.FETCH_LICENSE_KEY_FAILED:
      return state
        .setIn(['current', 'license', 'licenseFetchInProgress'], false)
        .setIn(['current', 'license', 'licenseKey'], null)
        .setIn(['current', 'license', 'expirationDate'], null)
        .setIn(['current', 'license', 'fetchedLicenseMessage'], action.payload.errorMessage || null)
        .setIn(['current', 'license', 'isValid'], false); // Set to false instead of null to prevent refetch

    default:
      return state;
  }
}
