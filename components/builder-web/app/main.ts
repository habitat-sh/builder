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

import 'reflect-metadata';
import 'zone.js';
import { platformBrowser } from '@angular/platform-browser';
import { enableProdMode } from '@angular/core';
import { AppModule } from './app.module';
import config from './config';

if (config['environment'] === 'production') {
  enableProdMode();
}

platformBrowser().bootstrapModule(AppModule)
  .catch(err => {
    console.error('Angular bootstrap error:', err);
    const msg = (err && (err.message || JSON.stringify(err))) || String(err);
    const stack = (err && err.stack) || '';
    document.body.style.cssText = 'background:#fff;color:#c00;padding:20px;font-family:monospace;font-size:13px;';
    document.body.innerHTML = '<h2>Bootstrap Error</h2><pre>' + msg + '\n\n' + stack + '</pre>';
  });
