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

import * as depotApi from '../client/depot-api';

export const CLEAR_EVENTS = 'CLEAR_EVENTS';
export const SET_VISIBLE_EVENTS = 'SET_VISIBLE_EVENTS';
export const SET_EVENTS_TOTAL_COUNT = 'SET_EVENTS_TOTAL_COUNT';
export const SET_EVENTS_NEXT_RANGE = 'SET_EVENTS_NEXT_RANGE';
export const SET_EVENTS_SEARCH_QUERY = 'SET_EVENTS_SEARCH_QUERY';
export const SET_EVENTS_DATE_FILTER = 'SET_EVENTS_DATE_FILTER';

export function fetchEvents(nextRange: number = 0, fromDate: string, toDate: string, query: string = '') {
  return dispatch => {
    if (nextRange === 0) {
      dispatch(clearEvents());
    }

    depotApi.getEvents(nextRange, fromDate, toDate, query).then(response => {
      dispatch(setVisibleEvents(response['results']));
      dispatch(setEventsTotalCount(response['totalCount']));
      dispatch(setEventsNextRange(response['nextRange']));
    }).catch(error => {
      dispatch(setVisibleEvents(undefined, error));
    });
  };
}

function clearEvents() {
  return {
    type: CLEAR_EVENTS,
  };
}

function setVisibleEvents(params, error = undefined) {
  return {
    type: SET_VISIBLE_EVENTS,
    payload: params,
    error: error,
  };
}

function setEventsTotalCount(payload: number) {
  return {
    type: SET_EVENTS_TOTAL_COUNT,
    payload,
  };
}

function setEventsNextRange(payload: number) {
  return {
    type: SET_EVENTS_NEXT_RANGE,
    payload,
  };
}

export function setEventsSearchQuery(payload: string) {
  return {
    type: SET_EVENTS_SEARCH_QUERY,
    payload,
  };
}

export function setEventsDateFilter(payload: any) {
  return {
    type: SET_EVENTS_DATE_FILTER,
    payload,
  };
}
