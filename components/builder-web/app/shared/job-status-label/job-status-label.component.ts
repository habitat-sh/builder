import { Component, Input } from '@angular/core';
import { iconForJobState, labelForJobState } from '../../util';

@Component({
  selector: 'hab-job-status-label',
  template: `<span [class]="classFor(job)" [title]="labelFor(job)">{{ labelFor(job) }}</span>`
})
export class JobStatusLabelComponent {

  @Input() job: object;

  private classFor({ state }) {
    if (state) {
      return state.toLowerCase();
    }
  }

  private labelFor({ state }) {
    if (state) {
      return labelForJobState(state);
    }
  }
}
