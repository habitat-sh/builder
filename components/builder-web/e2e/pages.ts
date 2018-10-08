import { Selector } from 'testcafe';

export class SignInPage {

  static get signInButton() {
    return Selector('.sign-in-page-component a.button');
  }
}
