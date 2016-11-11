// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

import React, { Component } from 'react';
import { observer } from 'mobx-react';

import Store from '../store';
import styles from './application.css';

import { api } from '../parity';

@observer
export default class Application extends Component {
  store = Store.get();

  render () {
    return this.store.loading
      ? this.renderLoading()
      : this.renderApp();
  }

  renderApp () {
    return (
      <div className={ styles.body }>
        <div className={ styles.warning }>
          WARNING: Registering a dapp is for developers only. Please ensure you understand the steps needed to develop and deploy applications, should you wish to use this dapp for anything apart from queries. A non-refundable fee of { api.util.fromWei(this.store.fee).toFormat(3) }<small>ETH</small> is required for any registration.
        </div>
      </div>
    );
  }

  renderLoading () {
    return (
      <div className={ styles.body }>
        <div className={ styles.loading }>Loading application</div>
      </div>
    );
  }
}
