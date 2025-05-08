import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule
  ],
  template: `
    <footer class="app-footer">
      <div class="copyright">
        Copyright © 2012-2025 Progress Software Corporation and/or its subsidiaries or affiliates. All Rights Reserved.
      </div>
      <div class="footer-links">
        <div class="help">
          Need help? If you have questions or you're stuck,
          <a href="https://www.chef.io/support" target="_blank">we're here to help</a>.
        </div>
        <div class="legal-links">
          <a href="https://www.chef.io/end-user-license-agreement" target="_blank">End User License Agreement</a>
          <a href="https://www.progress.com/legal/privacy-policy" target="_blank">Privacy Policy</a>
          <a href="https://www.progress.com/legal/cookie-policy" target="_blank">Cookie Policy</a>
        </div>
      </div>
    </footer>
  `,
  styleUrls: ['./footer.component.scss']
})
export class FooterComponent {}
