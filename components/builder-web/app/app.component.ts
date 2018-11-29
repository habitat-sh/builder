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

import config from './config';
import { Subscription } from 'rxjs';
import { AppStore } from './app.store';
import { Component, OnInit, OnDestroy } from '@angular/core';
import { URLSearchParams } from '@angular/http';
import { Router, NavigationStart } from '@angular/router';
import { identifyUser, loadFeatures, removeNotification, exchangeOAuthCode,
  routeChange, loadOAuthProvider, setPackagesSearchQuery, signOut, toggleUserNavMenu } from './actions/index';

const md5 = require('blueimp-md5');

@Component({
  selector: 'hab-app',
  template: require('./app.component.html')
})
export class AppComponent implements OnInit, OnDestroy {
  removeNotification: Function;
  signOut: Function;
  toggleUserNavMenu: Function;
  menuOpen: boolean = false;

  private sub: Subscription;

  constructor(private router: Router, private store: AppStore) {
    store.dispatch(loadFeatures());
    store.dispatch(loadOAuthProvider());

    // Whenever the Angular route has an event, dispatch an event with the new
    // route data.
    this.sub = this.router.events.subscribe(event => {
      if (event instanceof NavigationStart) {
        store.dispatch(routeChange(event));
      }

      // Clear the package search when the route changes
      store.dispatch(setPackagesSearchQuery(''));

      // Close the menu
      this.menuOpen = false;

      // Scroll to the top of the view
      window.scrollTo(0, 0);
    });

    // Listen for changes on the state.
    store.subscribe(state => {
      // If the state has a requestedRoute attribute, use the router to navigate
      // to the route that was requested.
      const requestedRoute = state.router.requestedRoute;
      if (requestedRoute) {
        router.navigate(requestedRoute);
      }
    });

    this.removeNotification = function (i) {
      this.store.dispatch(removeNotification(i));
      return false;
    }.bind(this);

    this.signOut = function () {
      this.store.dispatch(signOut(true));
      return false;
    }.bind(this);

    this.toggleUserNavMenu = function () {
      this.store.dispatch(toggleUserNavMenu());
      return false;
    }.bind(this);
  }

  get origin() {
    return this.state.origins.current;
  }

  get state() {
    return this.store.getState();
  }

  get avatarUrl() {
    const user = this.state.users.current;
    let url = '/assets/images/avatar.svg';

    if (config.use_gravatar && user.profile.email) {
      url = `https://secure.gravatar.com/avatar/${md5(user.profile.email.toLowerCase().trim())}?d=retro&s=40`;
    }

    return url;
  }

  get isSignedIn() {
    return !!this.state.session.token;
  }

  get isSigningIn() {
    return this.state.users.current.isSigningIn;
  }

  get isUserNavOpen() {
    return this.state.users.current.isUserNavOpen;
  }

  get username() {
    return this.state.users.current.profile.name;
  }

  ngOnDestroy() {
    this.sub.unsubscribe();
  }

  ngOnInit() {
    this.handleSignIn();
  }

  get layout() {
    return this.store.getState().ui.layout;
  }

  toggleMenu() {
    this.menuOpen = !this.menuOpen;
  }

  private handleSignIn() {
    const params = new URLSearchParams(window.location.search.slice(1));
    const code = params.get('code');
    const state = params.get('state');

    if (code) {
      this.store.dispatch(exchangeOAuthCode(code, state));
    }

    this.store.dispatch(identifyUser());
  }
}
