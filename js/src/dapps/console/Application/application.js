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

import { observer } from 'mobx-react';
import React, { Component } from 'react';

import { api } from '../parity';

import Console from '../Console';
import Header from '../Header';
import Input from '../Input';
import Status from '../Status';

import ApplicationStore from '../applicationStore';
import WatchesStore from '../watchesStore';

import styles from './application.css';

@observer
export default class Application extends Component {
  application = ApplicationStore.get();
  watches = WatchesStore.get();

  componentWillMount () {
    this.watches.addWatch('time', () => new Date());
    this.watches.addWatch('blockNumber', api.eth.blockNumber, api);
  }

  render () {
    return (
      <div className={ styles.app }>
        <div className={ styles.header }>
          <Header />
        </div>

        { this.renderView() }

        <div className={ styles.status }>
          <Status />
        </div>
      </div>
    );
  }

  renderView () {
    const { view } = this.application;

    if (view === 'console') {
      return (
        <div className={ styles.view }>
          <div className={ styles.eval }>
            <Console />
          </div>
          <div className={ styles.input }>
            <Input />
          </div>
        </div>
      );
    }

    return (
      <div className={ styles.view }>
        { view }
      </div>
    );
  }
}
