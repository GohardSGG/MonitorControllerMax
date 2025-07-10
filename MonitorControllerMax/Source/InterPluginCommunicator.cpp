/*
  ==============================================================================

    InterPluginCommunicator.cpp
    Created: 4 Aug 2024 10:00:00am
    Author:  Your Name

  ==============================================================================
*/

#include "InterPluginCommunicator.h"

InterPluginCommunicator::InterPluginCommunicator(MonitorControllerMaxAudioProcessor& p)
    : juce::InterprocessConnection(true, 0), processor(p) // Note: now uses a timer thread
{
    if (beginWaitingForSocket(ipcPort))
    {
        // Successfully became the server
        setRole(MonitorControllerMaxAudioProcessor::Role::master);
        DBG("IPC: Acting as MASTER");
    }
    else
    {
        // Couldn't become server, so try to connect as a client
        if (connectToSocket("127.0.0.1", ipcPort, 1000))
        {
            setRole(MonitorControllerMaxAudioProcessor::Role::slave);
            DBG("IPC: Acting as SLAVE");
        }
        else
        {
            setRole(MonitorControllerMaxAudioProcessor::Role::standalone);
            DBG("IPC: Acting as STANDALONE");
        }
    }
}

InterPluginCommunicator::~InterPluginCommunicator()
{
    stop();
    connectedClients.clear();
}

void InterPluginCommunicator::sendMuteSoloState(const MonitorControllerMaxAudioProcessor::MuteSoloState& state)
{
    if (processor.getRole() == MonitorControllerMaxAudioProcessor::Role::master)
    {
        juce::MemoryBlock mb;
        mb.append(&state, sizeof(state));

        for (auto* client : connectedClients)
            client->sendMessage(mb);
    }
}

juce::InterprocessConnection* InterPluginCommunicator::createConnectionObject()
{
    // A new client has connected to our server.
    return new ClientConnection(*this);
}

void InterPluginCommunicator::clientConnected(ClientConnection* client)
{
    connectedClients.push_back(client);
    DBG("IPC: New client connected. Total clients: " << connectedClients.size());
}

void InterPluginCommunicator::clientDisconnected(ClientConnection* client)
{
    // A client has disconnected. We need to find and remove it.
    auto it = std::find(connectedClients.begin(), connectedClients.end(), client);
    if (it != connectedClients.end())
    {
        connectedClients.erase(it);
        DBG("IPC: Client disconnected. Total clients: " << connectedClients.size());
    }
}

void InterPluginCommunicator::connectionMade()
{
    // Called when we (a client) successfully connect to the server
    setRole(MonitorControllerMaxAudioProcessor::Role::slave);
    DBG("IPC: Connection to master established.");
}

void InterPluginCommunicator::connectionLost()
{
    // If we are a slave, we should try to reconnect.
    if (processor.getRole() == MonitorControllerMaxAudioProcessor::Role::slave)
    {
        DBG("IPC: Connection to master lost. Attempting to reconnect...");
        // This will be handled by the timer in the base InterprocessConnection class
        // which will periodically try to reconnect.
        setRole(MonitorControllerMaxAudioProcessor::Role::standalone); // Revert to standalone until reconnected
    }
}

void InterPluginCommunicator::messageReceived(const juce::MemoryBlock& message)
{
    if (processor.getRole() == MonitorControllerMaxAudioProcessor::Role::slave)
    {
        if (message.getSize() == sizeof(MonitorControllerMaxAudioProcessor::MuteSoloState))
        {
            auto* state = static_cast<const MonitorControllerMaxAudioProcessor::MuteSoloState*>(message.getData());
            processor.setRemoteMuteSoloState(*state);
        }
    }
}

void InterPluginCommunicator::setRole(MonitorControllerMaxAudioProcessor::Role newRole)
{
    processor.setRole(newRole);
} 