import { Component, Input } from '@angular/core';
import { iconForJobState, labelForJobState } from '../../util';

@Component({
  standalone: false,
  selector: 'hab-job-status-label',
  template: `<span [class]="classFor(job)" [title]="labelFor(job)">{{ labelFor(job) }}</span>`
})
export class JobStatusLabelComponent {

  @Input() job: any;

  classFor({ state }: any) {
    if (state) {
      return state.toLowerCase();
    }
  }

  labelFor({ state }: any) {
    if (state) {
      return labelForJobState(state);
    }
  }
}
