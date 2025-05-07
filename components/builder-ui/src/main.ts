import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './app/app.config';
import { AppComponent } from './app/app.component';
import { environment } from './environments/environment';

// Log environment information during development
if (!environment.production) {
  console.log('Running in development mode with config:', environment);
}

bootstrapApplication(AppComponent, appConfig)
  .catch((err) => console.error(err));
