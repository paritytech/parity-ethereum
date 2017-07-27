// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import 'whatwg-fetch';

import es6Promise from 'es6-promise';
es6Promise.polyfill();

import Api from '@parity/api';
import Web3 from 'web3';

import web3extensions from './web3.extensions';

function initProvider () {
  let [, appId] = window.location.pathname.split('/');

  if (appId.indexOf('.html') !== -1) {
    appId = appId.replace('.html', '');
  }

  if (appId.substr(0, 2) !== '0x') {
    appId = Api.util.sha3(appId);
  }

  const ethereum = new Api.Provider.PostMessage(appId);

  console.log(`Requesting communications token for ${appId}`);

  ethereum
    .requestNewToken()
    .then((tokenId) => {
      console.log(`Received new communications token ${tokenId}`);
    })
    .catch((error) => {
      console.error('Unable to retrieve communications token', error);
    });

  global.ethereum = ethereum;
}

function initWeb3 () {
  // FIXME: Use standard provider for web3
  const http = new Web3.providers.HttpProvider('/rpc/');
  const web3 = new Web3(http);

  // set default account
  web3.eth.getAccounts((error, accounts) => {
    if (error || !accounts || !accounts[0]) {
      return;
    }

    web3.eth.defaultAccount = accounts[0];
  });

  web3extensions(web3).map((extension) => web3._extend(extension));

  global.web3 = web3;
}

function initParity () {
  const api = new Api(global.ethereum);

  global.parity = {
    Api,
    api
  };
}

initProvider();
initWeb3();
initParity();
