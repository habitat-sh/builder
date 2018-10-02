import { Component, Input } from '@angular/core';

@Component({
  selector: 'hab-job-status',
  template: `
    <hab-job-status-icon [job]="job" [animate]="true"></hab-job-status-icon>
    <hab-job-status-label [job]="job"></hab-job-status-label>
  `
})
export class JobStatusComponent {

  @Input() job: any;
}
