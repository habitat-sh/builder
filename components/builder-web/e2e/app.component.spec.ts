import { Selector } from 'testcafe';
import { config } from './config';
import { getCookie } from './util';
import { SignInPage } from './pages';

fixture('Sign-in')
  .page('http://localhost:5000/#/sign-in');

test('presents a properly configured sign-in button', async t => {
  const button = SignInPage.signInButton;
  const oauthState = await getCookie('oauthState');

  const params = [
    `client_id=${config.oauth_client_id}`,
    `redirect_uri=${encodeURIComponent(config.oauth_redirect_url)}`,
    `response_type=code`,
    `state=${oauthState}`
  ];

  const label = await button.textContent;
  const href = button.getAttribute('href');

  await t
    .expect(label.trim()).eql('Sign In with GitHub')
    .expect(href).eql(`${config.oauth_authorize_url}?${params.join('&')}`);
});
