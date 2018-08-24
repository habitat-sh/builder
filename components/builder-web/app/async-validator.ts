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

import { Observable } from 'rxjs';
import { Observer } from 'rxjs';
import { debounceTime, distinctUntilChanged, map } from 'rxjs/operators';
import { FormControl } from '@angular/forms';

// Wraps an async validator with a static `debounce` method, so you can debounce
// async validation.
//
// Where you would normally put:
//
//     myAsyncValidator
//
// Use:
//
//     AsyncValidator.debounce(control => myAsyncValidator(control))
//
// Taken from http://stackoverflow.com/a/36076946.
export class AsyncValidator {
  private validate: Function;

  constructor(validator: (control: FormControl) => any, time = 300) {

    let source: Observable<FormControl> = new Observable((observer: Observer<FormControl>) => {
      this.validate = (control) => observer.next(control);
    });

    source
      .pipe(
        debounceTime(time),
        distinctUntilChanged(null, (x: any) => x.control.value),
        map((x: any) => {
          return {
            promise: validator(x.control),
            resolver: x.promiseResolver
          };
        })
      )
      .subscribe(
        (x) => x.promise.then(
          resultValue => x.resolver(resultValue),
          e => console.log('async validator error: %s', e)
        )
      );
  }

  private getValidator() {
    return (control) => {
      let promiseResolver;

      let p = new Promise((resolve) => {
        promiseResolver = resolve;
      });

      this.validate({ control: control, promiseResolver: promiseResolver });
      return p;
    };
  }

  static debounce(validator: (control: FormControl) => any, debounceTime = 400) {
    const asyncValidator = new this(validator, debounceTime);
    return asyncValidator.getValidator();
  }
}
