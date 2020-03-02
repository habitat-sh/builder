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

import { TestBed, ComponentFixture } from '@angular/core/testing';
import { DebugElement, SimpleChange } from '@angular/core';
import { By } from '@angular/platform-browser';
import { MockComponent } from 'ng2-mock-component';
import { ReactiveFormsModule, FormGroup } from '@angular/forms';
import { CheckingInputComponent } from './checking-input.component';

describe('CheckingInputComponent', () => {
  let fixture: ComponentFixture<CheckingInputComponent>;
  let component: CheckingInputComponent;
  let element: DebugElement;

  const inputEl = () => fixture.nativeElement.querySelector('input');

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [
        ReactiveFormsModule
      ],
      declarations: [
        CheckingInputComponent,
        MockComponent({ selector: 'hab-icon', inputs: ['symbol'] })
      ]
    });

    fixture = TestBed.createComponent(CheckingInputComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;

    component.form = new FormGroup({});
  });

  it('can be disabled via `disabled` input property', () => {
    component.disabled = false;
    component.ngOnChanges();
    fixture.detectChanges();
    expect(inputEl().hasAttribute('disabled')).toBe(false);

    component.disabled = true;
    component.ngOnChanges();
    fixture.detectChanges();
    expect(inputEl().hasAttribute('disabled')).toBe(true);
  });
});
