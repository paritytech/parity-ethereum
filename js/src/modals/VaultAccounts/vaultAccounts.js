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
import React, { Component, PropTypes } from 'react';
import { FormattedMessage } from 'react-intl';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';

import { newError } from '~/redux/actions';
import { AccountCard, Button, Portal, SectionList } from '~/ui';
import { CancelIcon, CheckIcon } from '~/ui/Icons';

import styles from './vaultAccounts.css';

@observer
class VaultAccounts extends Component {
  static propTypes = {
    accounts: PropTypes.object.isRequired,
    newError: PropTypes.func.isRequired,
    vaultStore: PropTypes.object.isRequired
  }

  render () {
    const { accounts } = this.props;
    const { isBusyAccounts, isModalAccountsOpen, selectedAccounts } = this.props.vaultStore;

    if (!isModalAccountsOpen) {
      return null;
    }

    const vaultAccounts = Object
      .keys(accounts)
      .filter((address) => accounts[address].uuid)
      .map((address) => accounts[address]);

    return (
      <Portal
        buttons={ [
          <Button
            disabled={ isBusyAccounts }
            icon={ <CancelIcon /> }
            key='cancel'
            label={
              <FormattedMessage
                id='vaults.accounts.button.cancel'
                defaultMessage='Cancel'
              />
            }
            onClick={ this.onClose }
          />,
          <Button
            disabled={ isBusyAccounts }
            icon={ <CheckIcon /> }
            key='execute'
            label={
              <FormattedMessage
                id='vaults.accounts.button.execute'
                defaultMessage='Set'
              />
            }
            onClick={ this.onExecute }
          />
        ] }
        busy={ isBusyAccounts }
        onClose={ this.onClose }
        open
        title={
          <FormattedMessage
            id='vaults.accounts.title'
            defaultMessage='Manage Vault Accounts'
          />
        }
      >
        <SectionList
          items={ vaultAccounts }
          noStretch
          renderItem={ this.renderAccount }
          selectedAccounts={ selectedAccounts }
        />
      </Portal>
    );
  }

  renderAccount = (account) => {
    const { vaultName, selectedAccounts } = this.props.vaultStore;
    const isInVault = account.meta.vault === vaultName;
    const isSelected = isInVault
      ? !selectedAccounts[account.address]
      : selectedAccounts[account.address];

    const onSelect = () => {
      this.props.vaultStore.toggleSelectedAccount(account.address);
    };

    return (
      <div className={ styles.item }>
        <AccountCard
          account={ account }
          className={
            isSelected
              ? styles.selected
              : styles.unselected
          }
          onClick={ onSelect }
        />
        <div className={ styles.overlay }>
          {
            isSelected
              ? <CheckIcon onClick={ onSelect } />
              : <CheckIcon className={ styles.iconDisabled } onClick={ onSelect } />
          }
        </div>
      </div>
    );
  }

  onClose = () => {
    this.props.vaultStore.closeAccountsModal();
  }

  onExecute = () => {
    const { accounts } = this.props;
    const { vaultName, selectedAccounts } = this.props.vaultStore;

    const vaultAccounts = Object
      .keys(accounts)
      .filter((address) => accounts[address].uuid && selectedAccounts.includes(address))
      .map((address) => accounts[address]);

    return Promise
      .all([
        this.props.vaultStore.moveAccounts(
          vaultName,
          vaultAccounts
            .filter((account) => account.meta.vault !== vaultName)
            .map((account) => account.address)
        ),
        this.props.vaultStore.moveAccounts(
          null,
          vaultAccounts
            .filter((account) => account.meta.vault === vaultName)
            .map((account) => account.address)
        )
      ])
      .catch(this.props.newError)
      .then(this.onClose);
  }
}

function mapStateToProps (state) {
  const { accounts } = state.personal;

  return { accounts };
}

function mapDispatchToProps (dispatch) {
  return bindActionCreators({
    newError
  }, dispatch);
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(VaultAccounts);
