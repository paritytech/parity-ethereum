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

import React, { Component, PropTypes } from 'react';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';
import { observer } from 'mobx-react';
import { pick } from 'lodash';

import ActionDoneAll from 'material-ui/svg-icons/action/done-all';
import ContentClear from 'material-ui/svg-icons/content/clear';
import NavigationArrowBack from 'material-ui/svg-icons/navigation/arrow-back';
import NavigationArrowForward from 'material-ui/svg-icons/navigation/arrow-forward';

import { newError } from '~/ui/Errors/actions';
import { BusyStep, CompletedStep, Button, IdentityIcon, Modal, TxHash, Input } from '~/ui';
import { nullableProptype } from '~/util/proptypes';

import Details from './Details';
import Extras from './Extras';

import TransferStore from './store';
import styles from './transfer.css';

@observer
class Transfer extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired
  }

  static propTypes = {
    newError: PropTypes.func.isRequired,
    gasLimit: PropTypes.object.isRequired,
    images: PropTypes.object.isRequired,

    senders: nullableProptype(PropTypes.object),
    sendersBalances: nullableProptype(PropTypes.object),
    account: PropTypes.object,
    balance: PropTypes.object,
    wallet: PropTypes.object,
    onClose: PropTypes.func
  }

  store = new TransferStore(this.context.api, this.props);

  render () {
    const { stage, extras, steps } = this.store;

    return (
      <Modal
        actions={ this.renderDialogActions() }
        current={ stage }
        steps={ steps }
        waiting={ extras ? [2] : [1] }
        visible
      >
        { this.renderWarning() }
        { this.renderPage() }
      </Modal>
    );
  }

  renderAccount () {
    const { account } = this.props;

    return (
      <div className={ styles.hdraccount }>
        <div className={ styles.hdrimage }>
          <IdentityIcon
            address={ account.address }
            center
            inline />
        </div>
        <div className={ styles.hdrdetails }>
          <div className={ styles.hdrname }>
            { account.name || 'Unnamed' }
          </div>
          <div className={ styles.hdraddress }>
            { account.address }
          </div>
        </div>
      </div>
    );
  }

  renderPage () {
    const { extras, stage } = this.store;

    if (stage === 0) {
      return this.renderDetailsPage();
    } else if (stage === 1 && extras) {
      return this.renderExtrasPage();
    }

    return this.renderCompletePage();
  }

  renderCompletePage () {
    const { sending, txhash, busyState, rejected } = this.store;

    if (rejected) {
      return (
        <BusyStep
          title='The transaction has been rejected'
          state='You can safely close this window, the transfer will not occur.'
        />
      );
    }

    if (sending) {
      return (
        <BusyStep
          title='The transaction is in progress'
          state={ busyState } />
      );
    }

    return (
      <CompletedStep>
        <TxHash hash={ txhash } />
        {
          this.store.operation
          ? (
            <div>
              <br />
              <div>
                <p>This transaction needs confirmation from other owners.</p>
                <Input
                  style={ { width: '50%', margin: '0 auto' } }
                  value={ this.store.operation }
                  label='operation hash'
                  readOnly
                  allowCopy
                />
              </div>
            </div>
          )
          : null
        }
      </CompletedStep>
    );
  }

  renderDetailsPage () {
    const { account, balance, images, senders } = this.props;
    const { recipient, recipientError, sender, senderError, sendersBalances } = this.store;
    const { valueAll, extras, tag, total, totalError, value, valueError } = this.store;

    return (
      <Details
        address={ account.address }
        all={ valueAll }
        balance={ balance }
        extras={ extras }
        images={ images }
        onChange={ this.store.onUpdateDetails }
        recipient={ recipient }
        recipientError={ recipientError }
        sender={ sender }
        senderError={ senderError }
        senders={ senders }
        sendersBalances={ sendersBalances }
        tag={ tag }
        total={ total }
        totalError={ totalError }
        value={ value }
        valueError={ valueError }
        wallet={ account.wallet && this.props.wallet }
      />
    );
  }

  renderExtrasPage () {
    if (!this.store.gasStore.histogram) {
      return null;
    }

    const { isEth, data, dataError, total, totalError } = this.store;

    return (
      <Extras
        isEth={ isEth }
        data={ data }
        dataError={ dataError }
        total={ total }
        totalError={ totalError }
        gasStore={ this.store.gasStore }
        onChange={ this.store.onUpdateDetails } />
    );
  }

  renderDialogActions () {
    const { account } = this.props;
    const { extras, sending, stage } = this.store;

    const cancelBtn = (
      <Button
        icon={ <ContentClear /> }
        label='Cancel'
        onClick={ this.handleClose } />
    );
    const nextBtn = (
      <Button
        disabled={ !this.store.isValid }
        icon={ <NavigationArrowForward /> }
        label='Next'
        onClick={ this.store.onNext } />
    );
    const prevBtn = (
      <Button
        icon={ <NavigationArrowBack /> }
        label='Back'
        onClick={ this.store.onPrev } />
    );
    const sendBtn = (
      <Button
        disabled={ !this.store.isValid || sending }
        icon={ <IdentityIcon address={ account.address } button /> }
        label='Send'
        onClick={ this.store.onSend } />
    );
    const doneBtn = (
      <Button
        icon={ <ActionDoneAll /> }
        label='Close'
        onClick={ this.handleClose } />
    );

    switch (stage) {
      case 0:
        return extras
          ? [cancelBtn, nextBtn]
          : [cancelBtn, sendBtn];
      case 1:
        return extras
          ? [cancelBtn, prevBtn, sendBtn]
          : [doneBtn];
      default:
        return [doneBtn];
    }
  }

  renderWarning () {
    const { errorEstimated } = this.store.gasStore;

    if (!errorEstimated) {
      return null;
    }

    return (
      <div className={ styles.warning }>
        { errorEstimated }
      </div>
    );
  }

  handleClose = () => {
    const { onClose } = this.props;

    this.store.handleClose();
    typeof onClose === 'function' && onClose();
  }
}

function mapStateToProps (initState, initProps) {
  const { address } = initProps.account;

  const isWallet = initProps.account && initProps.account.wallet;
  const wallet = isWallet
    ? initState.wallet.wallets[address]
    : null;

  const senders = isWallet
    ? Object
      .values(initState.personal.accounts)
      .filter((account) => wallet.owners.includes(account.address))
      .reduce((accounts, account) => {
        accounts[account.address] = account;
        return accounts;
      }, {})
    : null;

  return (state) => {
    const { gasLimit } = state.nodeStatus;
    const sendersBalances = senders ? pick(state.balances.balances, Object.keys(senders)) : null;
    return { gasLimit, wallet, senders, sendersBalances };
  };
}

function mapDispatchToProps (dispatch) {
  return bindActionCreators({
    newError
  }, dispatch);
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(Transfer);
