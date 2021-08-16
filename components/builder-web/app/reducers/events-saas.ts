// Copyright (c) 2021 Chef Software Inc. and/or applicable contributors
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

import { List } from 'immutable';

import * as actionTypes from '../actions/index';
import initialState from '../initial-state';

export default function eventsSaas(state = initialState['eventsSaas'], action) {
  switch (action.type) {

    case actionTypes.CLEAR_SAAS_EVENTS:
      return state.set('nextRange', 0).
        set('visible', List()).
        set('totalCount', 0).
        setIn(['ui', 'visible', 'loading'], true).
        setIn(['ui', 'visible', 'exists'], false);

    case actionTypes.SET_SAAS_EVENTS_NEXT_RANGE:
      return state.set('nextRange', action.payload);

    case actionTypes.SET_SAAS_EVENTS_TOTAL_COUNT:
      return state.set('totalCount', action.payload);

    case actionTypes.SET_SAAS_EVENTS_SEARCH_QUERY:
      return state.set('searchQuery', action.payload);

    case actionTypes.SET_SAAS_EVENTS_DATE_FILTER:
      return state.set('dateFilter', action.payload);

    case actionTypes.SET_VISIBLE_SAAS_EVENTS:
      if (action.error) {
        return state.set('visible', List()).
          setIn(['ui', 'visible', 'errorMessage'], action.error.message).
          setIn(['ui', 'visible', 'exists'], false).
          setIn(['ui', 'visible', 'loading'], false);
      } else {
        return state.set('visible', state.get('visible').concat(List(action.payload))).
          setIn(['ui', 'visible', 'errorMessage'], undefined).
          setIn(['ui', 'visible', 'exists'], true).
          setIn(['ui', 'visible', 'loading'], false);
      }

    default:
      return state;
  }
}
