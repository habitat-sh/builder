import { Component, Input } from '@angular/core';
import { iconForJobState, labelForJobState } from '../../util';

@Component({
  selector: 'hab-job-status-icon',
  template: `<hab-icon [ngClass]="classes" [symbol]="symbol" [title]="label" [attr.title]="label"></hab-icon>`
})
export class JobStatusIconComponent {

  @Input() job: any;
  @Input() status: string;
  @Input() animate: boolean = false;

  private get _status() {
    return this.status || (this.job && this.job.state ? this.job.state : '');
  }

  private get classes() {
    let c = [this._status.toLowerCase()];

    if (this.animate) {
      c.push('animate');
    }

    return c;
  }

  private get symbol() {
    if (this._status) {
      return iconForJobState(this._status);
    }
  }

  private get label() {
    if (this._status) {
      return labelForJobState(this._status);
    }
  }
}
