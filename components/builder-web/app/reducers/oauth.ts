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

import initialState from '../initial-state';
import * as actionTypes from '../actions/index';

export default function oauth(state = initialState['oauth'], action) {
  switch (action.type) {

    case actionTypes.LOAD_OAUTH_STATE:
      return state
        .set('state', action.payload.state)
        .set('token', action.payload.token);

    case actionTypes.SET_OAUTH_PROVIDER:
      return state
        .setIn(['provider', 'type'], action.payload.type)
        .setIn(['provider', 'name'], action.payload.name)
        .setIn(['provider', 'clientID'], action.payload.clientID)
        .setIn(['provider', 'authorizeUrl'], action.payload.authorizeUrl)
        .setIn(['provider', 'redirectUrl'], action.payload.redirectUrl)
        .setIn(['provider', 'signupUrl'], action.payload.signupUrl)
        .setIn(['provider', 'useState'], action.payload.useState)
        .setIn(['provider', 'params'], action.payload.params);

    case actionTypes.SET_OAUTH_STATE:
      return state.set('state', action.payload);

    case actionTypes.SET_OAUTH_TOKEN:
      return state.set('token', action.payload);

    default:
      return state;
  }
}
