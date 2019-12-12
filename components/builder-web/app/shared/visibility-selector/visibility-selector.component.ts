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

import { Component, EventEmitter, Input, Output, OnInit } from '@angular/core';

export interface Option {
  title: string;
  description: string;
}

@Component({
  selector: 'hab-visibility-selector',
  template: require('./visibility-selector.component.html')
})

export class VisibilitySelectorComponent implements OnInit {

  @Input() visibilityContent?: any;
  @Input() setting: string = 'public';
  @Output() changed: EventEmitter<string> = new EventEmitter<string>();
  option1: Option = { title: '', description: ''};
  option2: Option = { title: '', description: '' };

  ngOnInit() {
    this.getOptions();
  }

  getOptions(): void {
    this.getOption1();
    this.getOption2();
  }

  getOption1(): void {
    this.option1.title = this.visibilityContent ? this.visibilityContent.option1.title : 'Public packages';
    this.option1.description = this.visibilityContent ? this.visibilityContent.option1.description : 'Package builds will appear in public search results and can be utilized by any user.';
  }

  getOption2(): void {
    this.option2.title = this.visibilityContent ? this.visibilityContent.option2.title : 'Private packages';
    this.option2.description = this.visibilityContent ? this.visibilityContent.option2.description : 'Package builds will NOT appear in public search results and can ONLY be utilized by members of this origin.';
  }

  change() {
    this.changed.emit(this.setting);
  }


}
