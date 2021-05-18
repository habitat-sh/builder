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

import { Component, HostListener, Input, OnChanges, OnDestroy, ElementRef, SimpleChanges } from '@angular/core';
import { Subscription } from 'rxjs';
import { default as AnsiUp } from 'ansi_up';
import * as moment from 'moment';
import { fetchJobLog, streamJobLog } from '../../actions/index';
import { iconForJobState, labelForJobState } from '../../util';
import { AppStore } from '../../app.store';

@Component({
  selector: 'hab-job-detail',
  template: require('./job-detail.component.html')
})
export class JobDetailComponent implements OnChanges, OnDestroy {
  @Input() job;
  @Input() stream: boolean = false;

  followLog: boolean = false;

  private jobSub: Function;
  private fetched: boolean = false;
  private lastJobState: string;
  private logSub: Subscription;
  private logHasContent: boolean = false;

  constructor(
    private store: AppStore,
    private elementRef: ElementRef) {
  }

  ngOnChanges(changes: SimpleChanges) {
    const job = changes['job'];

    if (job && job.currentValue && job.currentValue.id) {
      this.fetch(job.currentValue.id);
    }
  }

  ngOnDestroy() {
    if (this.logSub) {
      this.logSub.unsubscribe();
    }

    if (this.jobSub) {
      this.jobSub();
    }

    this.store.dispatch(streamJobLog(false));
  }

  @HostListener('window:scroll')
  @HostListener('window:resize')
  onScroll() { }

  @HostListener('window:wheel')
  onWheel() {
    this.followLog = false;
  }

  get controlsStyles() {
    let output = rectFor('.output.log');
    let controls = rectFor('.controls');
    let offsetY = window.innerHeight - output.top;
    let margin = 8;

    let props: any = {
      bottom: `${margin}px`
    };

    // To get the behavior we want (i.e., controls "pinned" to the bottom
    // of either the viewport or the output element), we switch between
    // fixed and absolute positioning, respectively.
    if (offsetY <= output.height) {
      props.position = 'fixed';
      props.left = `${output.right - controls.width - margin}px`;
    }
    else {
      props.position = 'absolute';
      props.right = `${margin}px`;
    }

    function rectFor(selector) {
      return document.querySelector(`.job-detail-component ${selector}`).getBoundingClientRect();
    }

    return props;
  }

  get jobState() {
    if (this.lastJobState) {
      return labelForJobState(this.lastJobState);
    }
  }

  get statusClass() {
    if (this.jobState) {
      return this.jobState.toLowerCase();
    }
  }

  toggleFollow() {
    this.followLog = !this.followLog;

    if (this.followLog) {
      this.scrollToEnd();
    }
  }

  get jobsLink() {
    return ['/pkgs', this.job.origin, this.job.name, 'jobs'];
  }

  get elapsed() {
    if (this.job) {
      let started = this.job.build_started_at;
      let finished = this.job.build_finished_at;
      let e;

      if (started && finished) {
        let s = +moment.utc(started);
        let f = +moment.utc(finished);
        e = moment.utc(f - s).format('m [min], s [sec]');
      }

      return e;
    }
  }

  get completed() {
    if (this.job) {
      let finished = this.job.build_finished_at;
      let f;

      if (finished) {
        f = moment.utc(finished).format('dddd, MMMM D, YYYY [at] h:mm:ss A');
      }

      return f;
    }
  }

  get ident() {
    if (this.job.origin && this.job.name && this.job.version && this.job.release) {
      return [
        this.job.origin, this.job.name, this.job.version, this.job.release
      ].join('/');
    }
  }

  get packageRoute() {
    if (this.ident) {
      return ['/pkgs', ...this.ident.split('/')];
    }
  }

  get info() {
    return this.store.getState().jobs.selected.info;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get showLog() {
    return this.fetched && this.logHasContent;
  }

  get showPending() {
    const log = this.store.getState().jobs.ui.selected.log;
    return !this.showLog && !log.loading && log.notFound;
  }

  public scrollToTop() {
    this.followLog = false;
    this.scrollTo(0);
  }

  public scrollToEnd() {
    let appHeight = this.elementHeight(this.container);
    this.scrollTo(appHeight - window.innerHeight);
  }

  public scrollTo(x = 0) {
    this.container.scrollTop = x;
  }

  public elementHeight(el) {
    return el ? el.scrollHeight : 0;
  }

  private element(selector) {
    return document.querySelector(selector);
  }

  private get container() {
    return this.element('.app main');
  }

  private fetch(id) {
    if (!this.fetched) {
      this.store.dispatch(streamJobLog(this.stream));
      this.store.dispatch(fetchJobLog(id, this.token, 0));
      this.watchStatus();
      this.watchLogs();
      this.fetched = true;
    }
  }

  private watchStatus() {
    this.jobSub = this.store.subscribe(state => {
      let s = state.jobs.selected.info.state;

      if (s && s !== this.lastJobState) {
        this.lastJobState = s;
      }
    });
  }

  private watchLogs() {
    let pre = this.elementRef.nativeElement.querySelector('pre');
    let content = this.store.getState().jobs.selected.log.content;

    this.logSub = content.subscribe((lines) => {

      if (lines.length > 0) {
        this.logHasContent = true;
      }

      let fragment = document.createDocumentFragment();
      const ansi_up = new AnsiUp();

      lines.forEach((line) => {
        let el = document.createElement('div');
        el.innerHTML = ansi_up.ansi_to_html(line);
        fragment.appendChild(el);
      });

      pre.appendChild(fragment);

      if (this.followLog) {
        this.scrollToEnd();
      }
    });
  }
}
