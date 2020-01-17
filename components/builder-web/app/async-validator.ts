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

import { AsyncValidatorFn, FormControl, ValidationErrors } from '@angular/forms';
import { from, Observable, timer } from 'rxjs';
import { switchMap } from 'rxjs/operators';

export class AsyncValidator {

  // Returns a new async validator that wraps provided async validator in a debounced observable.
  //
  // Where you would normally put:
  //
  //     myAsyncValidator
  //
  // Use:
  //
  //     AsyncValidator.debounce(myAsyncValidator);
  //     AsyncValidator.debounce(myAsyncValidator, 2000);
  static debounce(validatorFn: AsyncValidatorFn, debounceTime = 400): AsyncValidatorFn {
    return function debouncedAsyncValidator(control: FormControl): Observable<ValidationErrors> {
      return timer(debounceTime).pipe(switchMap(() => from(validatorFn(control))));
    };
  }
}
